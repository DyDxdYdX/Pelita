# Declarative extensions

Phase 3 adds local pack management. Phase 4 adds registry discovery and updates. Songbooks are the first supported type. Bible translations, constrained theme tokens and media collections are future types. A pack contains data only. Scripts, executable plugins, Rust dynamic libraries, arbitrary HTML and remote media references are excluded.

## Proposed v1 format

A .pelita-pack is a ZIP archive with a UTF-8 manifest at the root. Registry metadata is external to the archive.

```text
manifest.json
content/songbook.json
assets/001.mp3
licenses/content.txt
```

```json
{
  "schemaVersion": 1,
  "id": "org.example.rehearsal",
  "version": "1.0.0",
  "appCompatibility": ">=1.0.0 <2.0.0",
  "type": "songbook",
  "title": "Example rehearsal songs",
  "publisher": "Example",
  "entry": "content/songbook.json",
  "licenseFile": "licenses/content.txt",
  "files": [
    {"path": "content/songbook.json", "sha256": "<64 lowercase hex characters>"},
    {"path": "assets/001.mp3", "sha256": "<64 lowercase hex characters>"},
    {"path": "licenses/content.txt", "sha256": "<64 lowercase hex characters>"}
  ]
}
```

This example uses illustrative version bounds and digest placeholders. Require a recognized schema, reverse-domain ID, SemVer pack version, supported app version range, type, title, publisher, entry, license file and inventory of every file except the manifest. Each song record contains an immutable pack-scoped key, printed number, title, language tag, optional ordered lyric blocks and typed variants. Variants have stable keys and can reference only validated entries. Require attribution and origin records for rights-managed assets. License metadata is a publisher assertion. Reject unsupported schema versions, types, duplicate keys, missing references and unlisted files.

## Validation and lifecycle

1. Inspect archive metadata and manifest under resource limits. Reject absolute paths, traversal, links, special files, platform-reserved names and duplicate paths after Unicode and case normalization. Reject entries outside the inventory and restrict allowed formats.
2. Stream into an app-owned staging directory with limits on archive bytes, expanded bytes, per-file size, entry count, decoded dimensions and nesting. Proposed Phase 3 starting limits are 250 MiB compressed, 500 MiB expanded, 100 MiB per file and 5,000 files. Reconcile against the pilot corpus. Validate actual bytes, data schema and all digests before activation. Keep unpacked content inert.
3. Confirm publisher, origin, license and effects with the operator. Local archives are user-selected and unverified as to publisher identity. Registry installs must match the indexed ID, version and whole-archive SHA-256. Reject a same-version archive with a different digest.
4. Move the staged pack to an immutable version directory before switching the active version and index in one DB transaction. On interruption, retain the prior active version and clean abandoned staging or unreferenced directories at startup. Serialize installs per pack. Rollback selects a previously validated version. Retire unused versions only after referenced assets are released.
5. Enable or disable the pack transactionally and refresh search. Preview affected playlists and preserve their identifiers and labels. Keep already-live resources pinned until the operator moves on. Uninstall removes availability and later reclaims unreferenced files.

Phase 4 registry entries carry ID, version, title, type, publisher, app range, archive size, HTTPS URL, archive SHA-256 and license summary. Ship an allowlisted registry origin, preserve the last known valid index when refresh fails and verify downloads before installation. Hashes detect corruption relative to trusted metadata. Before public registry release decide how metadata is authenticated, who controls publishing, how keys or trusted origins are rotated, and how withdrawn packs are labeled. A registry outage or withdrawal must leave installed packs usable offline. See [local data](database-design.md) and [permissions](../legal/content-permissions.md).
