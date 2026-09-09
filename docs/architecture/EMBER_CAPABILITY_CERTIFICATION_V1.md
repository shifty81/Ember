# Ember Capability Certification V1

## Problem

Ember currently has descriptors that include capabilities which are planned but not yet executable.

Automation must never infer availability from a filename, crate presence or roadmap statement.

## State model

Every capability has exactly one state:

- `declared` — architecture reserves the name/shape
- `candidate` — implementation exists but required certification is incomplete
- `certified` — safe to advertise and execute
- `unavailable` — intentionally disabled or unsupported in this build/profile

## Capability record

```json
{
  "id": "editor.play_test",
  "version": 1,
  "state": "candidate",
  "owner": "ember_session",
  "mutation": true,
  "requires_approval": false,
  "platforms": ["windows"],
  "evidence": [],
  "dependencies": [
    "editor.session.launch",
    "runtime.session.handshake"
  ]
}
```

## Rules

1. Cortex handshake returns only certified executable capabilities by default.
2. Project Control Center status may expose all states for diagnostics.
3. UI commands may display candidate functions only when explicitly marked experimental.
4. A capability cannot become certified merely because its crate compiles.
5. Certification evidence is versioned and invalidated when its owning source changes.
6. Mutation capabilities must define transaction/rollback or clearly state why rollback is impossible.
7. Capability aliases are allowed only for compatibility and must identify their canonical replacement.

## Initial classification

### Certified today

- project root identification
- Cargo fmt/check/test/clippy through PCC
- editor build
- runtime-host build
- editor process launch
- runtime-host process launch
- Git authority inspection/repair
- FULL GREEN fingerprint gate
- GREEN commit/push
- root-drop patch validation/apply/rollback
- debug bundle
- source snapshot
- LDtk parse/convert unit-level contract
- basic level compile/runtime tick
- basic pixel document operations

### Candidate

- project-level validation
- asset validation
- editor workspace operations
- editor document save
- runtime capture
- Cortex adapter handshake/schema
- scene play-test workflow

### Declared

- native editor IPC
- named-pipe transport
- MCP transport
- package source/release
- shared Vault mounts
- full native Pixel/Animation workspace
- Terrain Composer
- native Levels workspace
- GUI Studio
- IDE workspace
- production export pipeline

Descriptors should be normalized to this truth model before Cortex begins consuming them as authoritative.
