# Ember-owned Cortex adapter

## Purpose

Cortex is a separate standalone application. Ember exposes project/editor capabilities to it without compiling against Cortex internals.

```text
Cortex
  |
  | plugin discovery + JSON-RPC/MCP-compatible messages
  v
Ember Cortex Adapter
  |---------------------> PCC machine API / CLI
  |---------------------> Ember editor IPC
  `---------------------> Ember project/document metadata
```

## Adapter responsibilities

- identify the Ember project and schema versions;
- expose safe project operations already owned by the Project Control Center;
- expose editor state/actions through Ember editor IPC;
- stream structured operation state/events;
- return artifacts/log/evidence references;
- enforce mutation approval/permission rules supplied by the host;
- remain usable when Cortex is upgraded independently.

## Initial tool surface

```text
ember.project.status
ember.project.health
ember.project.capabilities
ember.gate.fast
ember.gate.full
ember.build.foundry
ember.build.runtime
ember.test
ember.validate
ember.run.foundry
ember.run.runtime
ember.package.source
ember.package.release
ember.editor.status
ember.editor.open
ember.editor.open_document
ember.editor.active_document
ember.editor.selection
ember.editor.save
ember.editor.play_test
ember.assets.search
ember.assets.validate
ember.documents.validate
ember.logs.latest
ember.artifacts.list
```

Project operations are delegated to the Project Control Center. The adapter MUST NOT reimplement build, Git, patch, rollback, health, or packaging logic.

## Transport

The descriptor declares supported transports rather than hardcoding one implementation:

1. local JSONL/stdin-stdout process transport for simplest bootstrap;
2. local named-pipe JSON-RPC on Windows;
3. optional localhost HTTP/WebSocket for richer live events;
4. MCP capability projection when Cortex exposes its stable plugin SDK.

No transport may require public-network access for normal local editing.

## Version handshake

Every session starts with:

```json
{
  "protocol": "ember.cortex.adapter",
  "protocol_version": "1.0",
  "project_schema": "1",
  "capabilities": [],
  "pcc_contract": "1"
}
```

Cortex must negotiate capabilities and tolerate newer optional fields.

## Editor boundary

The adapter does not directly manipulate native editor internals. Ember editor exposes a typed project-local IPC surface such as:

```text
editor.status
editor.documents.list
editor.document.open
editor.document.save
editor.selection.get
editor.selection.set
editor.command.invoke
editor.play.start
editor.play.stop
editor.events.subscribe
```

This keeps the editor usable without Cortex and lets other trusted automation clients reuse the same interface later.
