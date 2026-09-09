# Ember Editor / Runtime Session Protocol V1

## Goal

Provide one local, typed, versioned contract for PIE and development runtime control.

The same public session surface may later be projected through Cortex, but Cortex is not required.

## Session lifecycle

```text
Editor
  -> create session
  -> launch runtime host
  -> protocol handshake
  -> runtime ready
  -> load project/scene
  -> play
  -> events/snapshots
  -> pause/step/hot-reload/capture as requested
  -> stop
  -> graceful shutdown
```

## Required message envelope

```json
{
  "protocol": "ember.session",
  "protocol_version": 1,
  "session_id": "session:...",
  "message_id": "...",
  "kind": "command|event|response",
  "name": "runtime.play",
  "timestamp": "...",
  "payload": {}
}
```

## Commands

Initial:

- `runtime.load`
- `runtime.play`
- `runtime.pause`
- `runtime.resume`
- `runtime.step`
- `runtime.stop`
- `runtime.reload_document`
- `runtime.reload_asset`
- `runtime.snapshot`
- `runtime.capture`
- `runtime.shutdown`

## Events

Initial:

- `runtime.starting`
- `runtime.ready`
- `runtime.loaded`
- `runtime.playing`
- `runtime.paused`
- `runtime.stopped`
- `runtime.snapshot`
- `runtime.capture_ready`
- `runtime.log`
- `runtime.diagnostic`
- `runtime.crashed`
- `runtime.disconnected`

## Transport

V1 transport order:

1. Windows named pipe for live local control.
2. stdio JSONL for simple process bootstrap/test harness.
3. file-backed snapshots/capture requests remain supported as fallback/evidence.

Do not require localhost/public network access for normal local PIE.

## Hot reload

Hot reload is document/asset oriented.

The editor sends a stable resource/document ID plus revision/fingerprint. Runtime either:

- accepts and applies,
- rejects with validation error,
- requests full scene reload.

Do not silently mutate runtime state from arbitrary editor memory.

## Parity rule

PIE and standalone runtime must consume the same compiled runtime representation.

Editor-only convenience paths must not create a second gameplay implementation.

## Failure behavior

- launch timeout -> session failed
- protocol mismatch -> fail closed with actionable versions
- runtime crash -> capture logs/snapshot if possible
- pipe disconnect -> session transitions to disconnected
- stale revision -> reject reload
- unsupported capability -> explicit unavailable response
