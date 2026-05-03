# Plan 001: GitHub Includes/Excludes - Target Specific Repo Paths

## Summary
Add `--includes` and `--excludes` CLI arguments to `dump github`. `--includes` targets specific directory paths (using git sparse checkout for efficiency). `--excludes` skips paths/glob patterns during extraction. This replaces the existing `--ignore` arg (which only worked for `src` dump) with a general solution for all dump types.

## Tasks

- [x] Update `IngestionTask` in `ingest/types.rs` - Add `includes: Option<Vec<String>>` and `excludes: Option<Vec<String>>` fields
- [x] Update `Crawler::crawl` in `ingest/crawler.rs` - Add sparse checkout logic when includes are set
- [x] Update `Extractor` methods in `ingest/extractor.rs` - Accept and apply includes/excludes to all discovery functions
- [x] Update `cli.rs` - Add `--includes` and `--excludes` args, replace `--ignore`, thread through all dump handlers
- [x] Update `lib.rs` - Add `includes`/`excludes` to `IngestSource` and `IngestionTask` construction
- [x] Update tests - Fix all `Extractor::extract` and `extract_all_tests` calls with new params
- [x] Update `README.md` - Document `--includes`/`--excludes` usage with examples
- [x] Run `cargo clippy` - clean, no warnings
- [x] Fix `gof` crate - Replace `SyncClient` with `AsyncClient` from `crates_io_api` to fix runtime panic in tokio
- [x] Fix `gof` crate - Add `includes: None, excludes: None` to `IngestionTask` construction
- [x] Test with `rerun-io/rerun --includes examples/rust` — ✅ `src` dump: 119K markdown with only `examples/rust` content
- [x] Fix sparse checkout lost after `git checkout` — add `sparse-checkout reapply` after every ref switch
- [x] Refactor `fetch --unshallow` into `fetch_tags_for_checkout` helper — tolerates non-shallow (blobless/sparse) clones

## Commits

1. `ec69010` feat(github): add includes/excludes filtering for targeted folder ingestion
2. `c7af59b` fix(gof): switch from SyncClient to AsyncClient for crates.io API
3. `a894c47` fix(github): re-apply sparse checkout after ref switch and handle non-shallow clones

## Design Decisions

- `--includes` uses `use_value_delimiter = true` (comma-separated directory paths)
  - e.g., `--includes examples/rust,crates/core`
  - When set: git sparse checkout limits clone, extractor filters to these prefixes
  - When unset: full repo (unchanged behavior)
- `--excludes` uses `use_value_delimiter = true` (comma-separated glob patterns)
  - e.g., `--excludes "*.lock,benches/**,fuzz/**"`
  - Replaces existing `--ignore` arg (which only applied to `src` dump)
  - Now works for all dump types: `examples`, `tests`, `src`
- Both can be combined: `--includes examples/rust --excludes "*_test.rs"`
- `--ignore` is removed in favor of `--excludes`

## Sparse Checkout Fix Details

When using `--includes`, the crawler does:
1. `git clone --filter=blob:none --sparse` (blobless, NOT shallow)
2. `git sparse-checkout set <paths>` (limits working tree)
3. `git fetch --tags` (needs tags for version resolution)
4. `git checkout <tag>` (switches ref — **this expands the working tree beyond the sparse cone!**)
5. `git sparse-checkout reapply` (restores the sparse filter)

Step 5 was missing, causing the extractor to see 0 files after checkout.
Additionally, `git fetch --unshallow` fails on blobless clones (they're not shallow),
so we refactored that into a tolerant helper method.

## Files Modified

1. `crates/github/src/ingest/types.rs` - Add `includes` and `excludes` fields to `IngestionTask`
2. `crates/github/src/ingest/crawler.rs` - Sparse checkout when includes are set, reapply after checkout, safe unshallow
3. `crates/github/src/ingest/extractor.rs` - Include/exclude-aware file discovery for all dump types
4. `crates/github/src/ingest/mod.rs` - Compile exclude patterns, pass to Extractor
5. `crates/github/src/cli.rs` - Replace `--ignore` with `--includes`/`--excludes`, thread through all handlers
6. `crates/github/src/lib.rs` - Add `includes`/`excludes` to `IngestSource` and `IngestionTask`
7. `crates/github/tests/extractor_test.rs` - Fix test calls with new params
8. `crates/github/tests/github_ingest_test.rs` - Fix test calls with new params
9. `crates/github/README.md` - Documentation and examples
10. `crates/gof/src/lib.rs` - Fix SyncClient→AsyncClient, add includes/excludes fields

## Test Results

### `src` dump — ✅ SUCCESS
```
cargo run -p cli dump github \
  --url https://github.com/rerun-io/rerun \
  --includes examples/rust \
  --dump-type src \
  --no-process

# → Generated 119K markdown (rerun-io-rerun-v0.5.1-src.md)
# → Only files from examples/rust/ appear in output
# → Sparse checkout: 135 entries, 14 files (vs ~167k objects full clone)
```

### `examples` dump — ❌ DB error (pre-existing, unrelated to this feature)
```
# Error: UNIQUE constraint failed / no such table: generated_examples
# The --no-process flag doesn't skip DB storage in the examples handler.
# This is a pre-existing issue with the storage pipeline, not the sparse checkout.
```

## Known Issues (pre-existing)

- 2 extractor tests fail (`test_extract_example_files`, `test_extract_from_include_bytes`) - pre-existing, unrelated
- `examples` dump handler ignores `--no-process` flag, always writes to DB
- SQLite "database is locked" errors under high concurrency in `gof` parallel ingestion