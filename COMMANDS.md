# PowerShell Commands Cheat Sheet

```powershell
# Node A
$env:WAL_PATH = ".\nodeA.wal"
$env:CHAOS_BEFORE_SYNC_MS = "1000"
Remove-Item Env:CHAOS_BEFORE_SYNC_MS

cargo run -- --node-id 1 --address http://127.0.0.1:3000 --leader-id 1 --peer-addresses http://127.0.0.1:3001,http://127.0.0.1:3002

Invoke-RestMethod -Method PUT "http://127.0.0.1:3000/key/x" -ContentType "application/json" -Body '{"value":"A"}'  

Invoke-RestMethod -Method DELETE "http://127.0.0.1:3000/key/x"

Invoke-RestMethod "http://127.0.0.1:3000/key/x"

Invoke-RestMethod "http://127.0.0.1:3000/ping"

Invoke-RestMethod "http://127.0.0.1:3000/health"

Invoke-RestMethod "http://127.0.0.1:3000/metrics"

Invoke-RestMethod -Method POST "http://127.0.0.1:3000/replicate" -ContentType "application/json" -Body '{"entries":[{"ts":1,"node_id":2,"operation":{"Put":{"key":"x","value":"B"}}}]}'


# Node B
$env:WAL_PATH = ".\nodeB.wal"
$env:CHAOS_BEFORE_SYNC_MS = "1000"
Remove-Item Env:CHAOS_BEFORE_SYNC_MS

cargo run -- --node-id 2 --address http://127.0.0.1:3001 --leader-id 1 --peer-addresses http://127.0.0.1:3000,http://127.0.0.1:3002

Invoke-RestMethod -Method PUT "http://127.0.0.1:3001/key/x" -ContentType "application/json" -Body '{"value":"B"}'  

Invoke-RestMethod -Method DELETE "http://127.0.0.1:3001/key/x"

Invoke-RestMethod "http://127.0.0.1:3001/key/x"

Invoke-RestMethod -Method POST "http://127.0.0.1:3001/replicate" -ContentType "application/json" -Body '{"entries":[{"ts":1,"node_id":1,"operation":{"Put":{"key":"x","value":"A"}}}]}'
```