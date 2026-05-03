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
- [x] Test with `rerun-io/rerun --includes examples/rust` — ✅ 56 examples extracted from `examples/rust` only
- [x] Fix directory pruning bug in `path_matches_filters` — ancestor dirs of include paths were incorrectly skipped

## Commits

1. `ec69010` feat(github): add includes/excludes filtering for targeted folder ingestion
2. `c7af19b` fix(gof): switch from SyncClient to AsyncClient for crates.io API
3. (pending) fix(github): ancestor directory pruning in path_matches_filters for includes

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

## Files Modified

1. `crates/github/src/ingest/types.rs` - Add `includes` and `excludes` fields to `IngestionTask`
2. `crates/github/src/ingest/crawler.rs` - Sparse checkout when includes are set
3. `crates/github/src/ingest/extractor.rs` - Include/exclude-aware file discovery for all dump types
4. `crates/github/src/ingest/mod.rs` - Compile exclude patterns, pass to Extractor
5. `crates/github/src/cli.rs` - Replace `--ignore` with `--includes`/`--excludes`, thread through all handlers
6. `crates/github/src/lib.rs` - Add `includes`/`excludes` to `IngestSource` and `IngestionTask`
7. `crates/github/tests/extractor_test.rs` - Fix test calls with new params
8. `crates/github/tests/github_ingest_test.rs` - Fix test calls with new params
9. `crates/github/README.md` - Documentation and examples
10. `crates/gof/src/lib.rs` - Fix SyncClient→AsyncClient, add includes/excludes fields

## Test Results

```
cargo run -p cli -- dump github \
  --url https://github.com/rerun-io/rerun \
  --includes examples/rust \
  --dump-type examples \
  --version main \
  --no-process

# → 56 unique examples, 7213 lines markdown
# → Sparse checkout: only 14 files + 135 dir entries (vs full ~167k objects)
```

## Known Issues (pre-existing)

- 2 extractor tests fail (`test_extract_example_files`, `test_extract_from_include_bytes`) - pre-existing, unrelated
- SQLite "database is locked" errors under high concurrency in `gof` parallel ingestion