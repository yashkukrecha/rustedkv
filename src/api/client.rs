use std::{collections::HashMap, sync::Arc, time::{SystemTime, UNIX_EPOCH}};
use prometheus::{Encoder, TextEncoder};
use axum::{
    extract::{Path, State},
    routing::{get, post, put, delete},
    Json, Router, response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use crate::api::{ApiState, Metrics};
use crate::store::{LamportClock};
use crate::util::{LogEntry, Operation};

pub struct RouterBuilder;

impl RouterBuilder {
    pub fn with_state(state: ApiState) -> Router {
        Router::new()
            .route("/key/:key", put(put_key).get(get_key).delete(delete_key))
            .route("/replicate", post(replicate))
            .route("/ping", get(ping))
            .route("/metrics", get(metrics))
            .with_state(state)
    }
}

#[derive(Serialize, Deserialize)]
pub struct PutBody {
    pub value: String,
}


async fn put_key(
    State(state): State<ApiState>,
    Path(key): Path<String>,
    Json(body): Json<PutBody>,
) -> Response {
    let ts = state.clock.tick_send();
    let node_id = state.cluster.read().await.node_id;
    let log_entry = LogEntry {
        ts: ts,
        node_id: node_id,
        operation: Operation::Put { key: key.clone(), value: body.value.clone() },
    };

    {
        let mut wal = state.wal.lock().await;

        // TESTING: chaos injection
        // wal.append(&log_entry).unwrap();
        // if state.chaos_before_sync_ms > 0 {
        //     tokio::time::sleep(std::time::Duration::from_millis(state.chaos_before_sync_ms)).await;
        // }
        // wal.sync().unwrap();

        wal.append_sync(&log_entry).unwrap();
    }

    let store = state.store;
    store.put(key, body.value, ts, node_id).await;

    state.metrics.requests.with_label_values(&["PUT", "/key/:key", "200"]).inc();
    state.metrics.kv_ops.with_label_values(&["put"]).inc();

    (axum::http::StatusCode::OK).into_response()
}

#[derive(Serialize, Deserialize)]
pub struct GetResp {
    value: Option<String>,
    ts: u64,
}

async fn get_key(
    State(state): State<ApiState>,
    Path(key): Path<String>,
) -> Response {
    let store = state.store;

    let (status, resp): (axum::http::StatusCode, Response) = if let Some(val) = store.get(&key).await {
        state.metrics.kv_ops.with_label_values(&["get"]).inc();
        let body = GetResp { value: val.data.clone(), ts: val.ts };
        (axum::http::StatusCode::OK, (axum::http::StatusCode::OK, Json(body)).into_response())
    } else {
        state.metrics.errors.with_label_values(&["not_found"]).inc();
        (axum::http::StatusCode::NOT_FOUND, axum::http::StatusCode::NOT_FOUND.into_response())
    };

    let status_label = if status == axum::http::StatusCode::OK { "200" } else { "404" };
    state.metrics.requests.with_label_values(&["GET", "/key/:key", status_label]).inc();

    resp
}

async fn delete_key(
    State(state): State<ApiState>,
    Path(key): Path<String>,
) -> Response {
    let ts = state.clock.tick_send();
    let node_id = state.cluster.read().await.node_id;
    let log_entry = LogEntry {
        ts: ts,
        node_id: node_id,
        operation: Operation::Delete { key: key.clone() },
    };

    {
        let mut wal = state.wal.lock().await;

        // TESTING: chaos injection
        // wal.append(&log_entry).unwrap();
        // if state.chaos_before_sync_ms > 0 {
        //     tokio::time::sleep(std::time::Duration::from_millis(state.chaos_before_sync_ms)).await;
        // }
        // wal.sync().unwrap();

        wal.append_sync(&log_entry).unwrap();
    }

    let store = state.store;
    let (status, resp): (axum::http::StatusCode, Response) = if let Some(_val) = store.delete(&key, ts, node_id).await {
        state.metrics.kv_ops.with_label_values(&["delete"]).inc();
        (axum::http::StatusCode::OK, axum::http::StatusCode::OK.into_response())
    } else {
        state.metrics.errors.with_label_values(&["not_found"]).inc();
        (axum::http::StatusCode::NOT_FOUND, axum::http::StatusCode::NOT_FOUND.into_response())
    };

    let status_label = if status == axum::http::StatusCode::OK { "200" } else { "404" };
    state.metrics.requests.with_label_values(&["DELETE", "/key/:key", status_label]).inc();

    resp
}

#[derive(Debug, Serialize, Deserialize)]
struct ReplicateBody {
    entries: Vec<LogEntry>
}

async fn replicate(
    State(state): State<ApiState>,
    Json(body): Json<ReplicateBody>,
) -> Response {
    for entry in body.entries {
        state.clock.tick_observe(entry.ts);

        {
            let mut wal = state.wal.lock().await;
            wal.append_sync(&entry).unwrap();
        }

        match entry.operation {
            Operation::Put { key, value } => {
                state.store.put(key, value, entry.ts, entry.node_id).await;
                state.metrics.kv_ops.with_label_values(&["put"]).inc();
            }
            Operation::Delete { key } => {
                state.store.delete(&key, entry.ts, entry.node_id).await;
                state.metrics.kv_ops.with_label_values(&["delete"]).inc();
            }
        }
    }

    state.metrics.requests.with_label_values(&["POST", "/replicate", "200"]).inc();
    axum::http::StatusCode::OK.into_response()
}

async fn ping() -> Response {
    (axum::http::StatusCode::OK, "pong").into_response()
}

async fn metrics(State(state): State<ApiState>) -> Response {
    let mut buffer = Vec::new();
    let enc = TextEncoder::new();
    enc.encode(&state.metrics.registry.gather(), &mut buffer).unwrap();
    (
        axum::http::StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, enc.format_type().to_string())],
        buffer,
    ).into_response()
}