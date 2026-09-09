# Ember Documents, Storage and Packages V1

## Documents

`DocumentEnvelope` remains the common registry wrapper, but concrete domain data must be typed and versioned.

Each concrete document type provides:

- canonical kind ID
- schema version
- codec
- validator
- migration functions
- dependency/reference enumeration
- optional runtime compiler
- optional editor workspace binding

Avoid making `serde_json::Value` the long-term domain authority.

## Save transaction

All editor-owned writes follow:

```text
serialize
 -> validate serialized candidate
 -> write temporary sibling
 -> flush/best effort durability
 -> atomic replace
 -> update revision/fingerprint
 -> emit saved event
```

Crash-recovery data is separate from canonical project files.

## References

Cross-document references use stable IDs plus optional expected kind.

Validation resolves:

- missing target
- wrong kind
- dependency cycle where prohibited
- stale package
- unavailable provider
- incompatible schema version

## Packages

A package is the replaceable unit for reusable systems/content.

Minimum manifest:

```json
{
  "schema_version": 1,
  "id": "package:example",
  "version": "1.0.0",
  "dependencies": [],
  "content_roots": [],
  "capabilities": [],
  "providers": [],
  "migrations": []
}
```

Package resolution happens before documents/assets are considered fully valid.

## Mounts

Project-local and shared-library content use mount records rather than copied absolute paths.

Mount kinds:

- project
- package
- vault
- generated/cache (non-authoritative)

A serialized game project must never depend on a developer-machine absolute path.

## Source / derived authority

Every imported/generated production asset records:

- stable asset ID
- provider
- original/source locator
- local authoritative file
- derived outputs
- source revision/hash
- license/provenance
- dependencies

Generated images and external-library previews remain artifacts until explicitly promoted.

## Vault

The Ember Vault is a provider/mount, not a hidden second asset system.

Projects reference versioned Vault records and may pin/promote/copy when portability requires it.
