use std::{collections::HashMap, sync::Arc, time::{SystemTime, UNIX_EPOCH, Duration}};
use prometheus::{Encoder, TextEncoder};
use axum::{
    extract::{Path, State},
    routing::{get, post, put, delete},
    Json, Router, response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use crate::api::{ApiState, Metrics};
use crate::store::{LamportClock, Value};
use crate::util::{LogEntry, Operation};
use crate::replication::ReplicateBody;
use crate::cluster::{quorum_write, quorum_read};

pub struct RouterBuilder;

impl RouterBuilder {
    pub fn with_state(state: ApiState) -> Router {
        Router::new()
            .route("/key/:key", put(put_key).get(get_key).delete(delete_key))
            .route("/replicate", post(replicate))
            .route("/ping", get(ping))
            .route("/health", get(health))
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

        // tests chaos injection
        wal.append(&log_entry).unwrap();
        if state.chaos_before_sync_ms > 0 {
            tokio::time::sleep(Duration::from_millis(state.chaos_before_sync_ms)).await;
        }
        wal.sync().unwrap();
    }

    let store = state.store;
    store.put(key, body.value, ts, node_id).await;

    let cluster = state.cluster.read().await;
    let peers = cluster.peer_addresses.clone();
    let total_nodes = peers.len() + 1;
    let write_quorum = (total_nodes / 2) + 1;
    let leader_id = cluster.leader_id;
    drop(cluster);

    if node_id == leader_id {
        if let Err(()) = quorum_write(log_entry.clone(), peers, write_quorum).await {
            state.metrics.errors.with_label_values(&["quorum_write"]).inc();
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
        state.rep_tx.send(log_entry).await;
    }

    state.metrics.requests.with_label_values(&["PUT", "/key/:key", "200"]).inc();
    state.metrics.kv_ops.with_label_values(&["put"]).inc();

    (axum::http::StatusCode::OK).into_response()
}

#[derive(Serialize, Deserialize)]
pub struct GetResp {
    data: Option<String>,
    ts: u64,
    node_id: u64,
}

async fn get_key(
    State(state): State<ApiState>,
    Path(key): Path<String>,
) -> Response {
    let store = state.store;
    let local_value = match store.get(&key).await {
        Some(val) => val,
        None => {
            let ts = 0;
            let node_id = state.cluster.read().await.node_id;
            Value { data: None, ts, node_id }
        }
    };

    let cluster = state.cluster.read().await;
    let peers = cluster.peer_addresses.clone();
    let total_nodes = peers.len() + 1;
    let read_quorum = (total_nodes / 2) + 1;
    let leader_id = cluster.leader_id;
    let node_id = cluster.node_id;
    drop(cluster);

    if node_id != leader_id {
        let body = GetResp { data: local_value.data, ts: local_value.ts, node_id: local_value.node_id };
        return (axum::http::StatusCode::OK, Json(body)).into_response();
    }

    let result = quorum_read(key.clone(), local_value.clone(), peers, read_quorum).await;
    match result {
        Ok(fresh) => {
            let status = if fresh.data.is_some() {
                axum::http::StatusCode::OK
            } else {
                axum::http::StatusCode::NOT_FOUND
            };
            state.metrics.kv_ops.with_label_values(&["get"]).inc();
            state.metrics.requests.with_label_values(&["GET", "/key/:key", "200"]).inc();
            let body = GetResp { data: fresh.data, ts: fresh.ts, node_id: fresh.node_id };
            (status, Json(body)).into_response()
        },
        Err(()) => {
            state.metrics.errors.with_label_values(&["quorum_read"]).inc();
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR).into_response()
        }
    }
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

        // tests chaos injection
        wal.append(&log_entry).unwrap();
        if state.chaos_before_sync_ms > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(state.chaos_before_sync_ms)).await;
        }
        wal.sync().unwrap();
    }

    let store = state.store;
    let result = store.delete(&key, ts, node_id).await;
    let (status, resp) = if result.is_some() {
        state.metrics.kv_ops.with_label_values(&["delete"]).inc();
        (axum::http::StatusCode::OK, axum::http::StatusCode::OK.into_response())
    } else {
        state.metrics.errors.with_label_values(&["not_found"]).inc();
        (axum::http::StatusCode::NOT_FOUND, axum::http::StatusCode::NOT_FOUND.into_response())
    };

    let cluster = state.cluster.read().await;
    let peers = cluster.peer_addresses.clone();
    let total_nodes = peers.len() + 1;
    let write_quorum = (total_nodes / 2) + 1;
    let leader_id = cluster.leader_id;
    drop(cluster);

    if node_id == leader_id {
        if let Err(()) = quorum_write(log_entry.clone(), peers, write_quorum).await {
            state.metrics.errors.with_label_values(&["quorum_write"]).inc();
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
        state.rep_tx.send(log_entry).await;
    }

    let status_label = if status == axum::http::StatusCode::OK { "200" } else { "404" };
    state.metrics.requests.with_label_values(&["DELETE", "/key/:key", status_label]).inc();

    resp
}

async fn replicate(
    State(state): State<ApiState>,
    Json(body): Json<ReplicateBody>, 
) -> Response {

    // only add fresh entries to WAL and store
    let mut to_apply = Vec::with_capacity(body.entries.len());
    for entry in &body.entries {
        match &entry.operation {
            Operation::Put { key, value } => {
                let cur = state.store.get(key).await;
                let incoming = Value { data: None, ts: entry.ts, node_id: entry.node_id };
                if incoming.is_newer_than(cur.as_ref()) {
                    to_apply.push(entry.clone());
                }
            }
            Operation::Delete { key } => {
                let cur = state.store.get(key).await;
                let incoming = Value { data: None, ts: entry.ts, node_id: entry.node_id };
                if incoming.is_newer_than(cur.as_ref()) {
                    to_apply.push(entry.clone());
                }
            }
        }
    }

    if to_apply.is_empty() {
        state.metrics.requests.with_label_values(&["POST", "/replicate", "200"]).inc();
        return (axum::http::StatusCode::OK).into_response();
    }

    {
        let mut wal = state.wal.lock().await;
        for entry in &to_apply {
            wal.append(&entry).unwrap();
        }
        wal.sync().unwrap();
    }

    for entry in to_apply {
        state.clock.tick_recv(entry.ts);
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

async fn health(State(state): State<ApiState>) -> Response {
    let is_alive = state.cluster.read().await.is_alive.clone();
    (axum::http::StatusCode::OK, axum::Json(is_alive)).into_response()
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