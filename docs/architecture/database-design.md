# Local data design

The Phase 1 pilot uses a [versioned service document and managed media files](../development/phase-1.md). The SQLite model below is proposed for the searchable library. Keep media bytes outside SQLite and user-authored data separate from installed pack data. Preserve stable IDs when migrating pilot services.

| Entity | Fields and invariants |
| --- | --- |
| Settings | Schema-backed preferences, preferred display descriptor and output volume. |
| Asset | ID, origin, media kind, managed relative path or user-linked path, size, digest when available. Paths resolve in Rust and missing sources remain repairable. |
| Deck / deck frame | Immutable prepared deck revision, optional source asset, renderer/options, ordered page references or text. Frame order unique within a deck. |
| Playlist / playlist item | Stable IDs, title, timestamps, unique order per playlist, display label, typed content reference and optional audio. Library items pin a variant ID. Use an explicit missing state when the source becomes unavailable. |
| Collection / song / stanza | Collection identity, source ownership, song number as text, title, language, ordered optional lyric blocks. Index normalized number per collection without losing the printed form. |
| Song variant / preference | Stable song reference and typed text, deck-with-optional-audio, or video payload. Preference references a variant of the same song. |
| Pack / pack version | Publisher-qualified ID, active version, enabled state, digest, install origin, and stable source record keys. Resolve source keys across compatible updates. |

Use transactional reorder and duplication. Assign a new ID to each duplicated item. Snapshot the prepared item for a Take and pin its media until transport releases it. Prevent deletions from cascading into playlists. Disabled or removed pack items become unresolved with their original IDs and labels. Offer relink, replacement or skip.

Propose FTS5 for title and lyrics with a separate exact-number index. Bind queries, quote or tokenize literal input, rank exact number then title then lyrics, and use stable sorting for ties. Update searchable rows transactionally with source edits and exclude disabled packs. Add Unicode and punctuation fixtures. [SQLite FTS5](https://www.sqlite.org/fts5.html)

Use one serialized writer, enable foreign key checks on every connection, and apply forward migrations transactionally. Back up the DB and referenced managed assets before incompatible migration using a consistent snapshot. Stage asset writes on the same volume before publishing DB references, reconcile unreferenced files after interruption, and defer deletion while any active item or backup needs a file. A linked file stays at its original location and may need relinking after restore. Managed copies are the proposed default for pilot reliability. Restore to a clean data directory, validate references, and test recovery before removing the prior backup. Persist only committed edits. A restarted application opens with projection cleared.
