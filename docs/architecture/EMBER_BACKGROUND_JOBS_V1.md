# Ember Background Job System V1

## Hard requirement

No long-running operation may block the native editor UI thread.

This includes:

- builds
- tests
- validation
- imports
- exports
- indexing
- asset conversion
- image generation
- model operations
- world generation/bakes
- runtime packaging
- downloads
- external tool processes

## Job record

```json
{
  "id": "job:...",
  "kind": "asset.import",
  "owner": "ember_assets",
  "state": "running",
  "progress": 0.42,
  "status": "Building atlas",
  "cancellable": true,
  "parent": null,
  "artifacts": [],
  "diagnostics": []
}
```

## States

- queued
- running
- succeeded
- failed
- cancelled

## Required service API

- submit
- cancel
- status
- list active
- subscribe events
- attach log
- attach artifact
- child job
- await from non-UI worker/test code

## UI behavior

The shell exposes one shared job surface:

- compact active-job indicator
- detailed jobs panel
- progress
- cancellation
- logs
- artifacts
- failure diagnostics

Individual workspaces do not invent their own background-task infrastructure.

## Process jobs

External processes are hosted by the job system with:

- stdout/stderr streaming
- exit code
- cancellation/termination policy
- working directory
- environment overrides
- artifact discovery
- structured timing

The PCC may remain synchronous as a CLI, but editor invocation of PCC/project operations occurs through a background job.
