# minigraf-binding-template

Template repository for building a new [Minigraf](https://github.com/project-minigraf/minigraf) language binding.

## What's in this repo

| File | Purpose |
|---|---|
| `Cargo.toml` | Rust shim crate — depends on `minigraf` core, produces a cdylib |
| `src/lib.rs` | The UniFFI shim shared by the Python, Java, Android and Swift bindings: open options, query cursors, prepared queries, the fact log and the log writer |
| `uniffi.toml` | Kotlin renames for the shim's `close()` methods |
| `src/uniffi_bindgen.rs` | Entry point for the `uniffi-bindgen` CLI binary |
| `.github/workflows/ci.yml` | CI: `cargo test`, then package, consumer test and readme-sync |
| `.github/workflows/package.yml` | The only place artifacts are built (placeholder: the cdylib) |
| `.github/workflows/verify-published.yml` | Post-publish consumer test from the registry; opens a `release-broken` issue on failure |
| `consumer-test/run.sh` | Installs the artifacts into an empty project and runs the quick start and the contract |
| `consumer-test/readme_sync.py` | Checks that the README quick start is what the consumer test runs (canonical copy) |

## Creating a new binding

1. Click **Use this template** on GitHub to create your repo under `project-minigraf/<language>`.
2. Update `Cargo.toml`: rename the crate, pin `minigraf` to the version you're targeting.
3. Run `cargo test` to verify the Rust layer compiles and tests pass.
4. Add your language tooling (e.g. `java/`, `android/`, `Sources/`, etc.).
5. Generate language bindings with `uniffi-bindgen`:
   ```bash
   cargo build --release
   cargo run --bin uniffi-bindgen -- generate \
     --library target/release/libminigraf_ffi.<so|dylib|dll> \
     --language <kotlin|swift|python|...> \
     --out-dir <output-dir>/
   ```
6. Fill in the packaging and the consumer test (next section).
7. Add a `release.yml` that receives the `core-release` repository_dispatch event from
   the minigraf cascade, pins the new `minigraf` version in `Cargo.toml`, commits, tags,
   and publishes your artifact, wired as described below.

## Packaging and the consumer test

Every binding proves, on every PR and in every release, that its package installs and
works the way a user installs it. Search for `TODO(binding)` to find every placeholder.

1. **`package.yml`** builds each artifact exactly as it is published, stamped with the
   `version` input (`0.0.0-ci` on PRs), checks its contents (native library, generated
   bindings, `LICENSE-MIT`, `LICENSE-APACHE`) and uploads it as `dist-<name>`. Nothing else
   builds release artifacts.
2. **`consumer-test/run.sh local <dist> <version>`** installs those artifacts into a fresh
   temporary project with the package manager, then runs `consumer-test/quickstart.<ext>`
   (the README quick start) and a contract test: in memory, file-backed with a reopen, the
   error type with its code, and the installed version. `registry <version>` does the same
   from the public registry, waiting up to 10 minutes for the version to appear. Resolve
   the dist path before `cd`, and pass relative paths to tools on Windows.
3. **`README.md`** gets a `## Quick start` section. readme-sync takes its first code block
   and requires every non-blank line, trimmed, to appear in the same order in
   `consumer-test/quickstart.<ext>`. The consumer may add imports, a wrapper and assertions
   around them. Set the file name in `ci.yml`. If the README covers more than one package,
   pass the section heading as a third argument.
4. **`ci.yml`**: one `consumer` leg per platform you publish for, downloading the matching
   `dist-<name>`.
5. **`release.yml`**:
   - the `test` job calls `ci.yml` with the pinned `sha` as `ref` and the release version as
     `version`, so the artifacts are built and consumer-tested before anything is tagged;
   - `publish` downloads `dist-*` from the same run and uploads exactly those files, never a
     rebuild, and skips a version the registry already has, so re-runs are safe;
   - a last job calls `verify-published.yml` with the version and the release tag.
6. **`verify-published.yml`**: the same matrix in registry mode. On failure it opens (or
   comments on) an issue labelled `release-broken`, and the run fails. Run it by hand with
   `gh workflow run verify-published.yml -f version=<x.y.z>`.

The canonical `readme_sync.py` lives here; copy it unchanged.

## The shared surface

Every binding exposes the same objects, specified in minigraf's
[`docs/superpowers/specs/2026-10-07-binding-etl-surface-design.md`](https://github.com/project-minigraf/minigraf/blob/v3/docs/superpowers/specs/2026-10-07-binding-etl-surface-design.md):

- `MiniGrafDb.open_with_options(path, OpenOptions)`, `query()` → `MiniGrafCursor`,
  `fact_log(FactFilter)` → `MiniGrafFactLog`, `current_tx_count()`;
- `MiniGrafLogWriter` (`create`, `append`, `append_batch`, `advance_tx_count`,
  `tx_count`, `finish`, `close`, `is_open`);
- `MiniGrafDb.prepare(text)` → `MiniGrafPreparedQuery`, with `execute(bindings)` (JSON)
  and `query(bindings)` (`MiniGrafCursor`); bindings are a map from slot name (no `$`)
  to `MiniGrafBindValue` (`Value`, `Entity`, `TxCount`, `Timestamp`, `AnyValidTime`).
  A prepared query keeps its database open until both are freed. Specified in
  [`docs/superpowers/specs/2026-10-24-prepared-query-ffi-design.md`](https://github.com/project-minigraf/minigraf/blob/v3/docs/superpowers/specs/2026-10-24-prepared-query-ffi-design.md);
  each language adds plain host values on top (§4.2);
- `FactRecord` with a lossless `MiniGrafValue`; counts are signed 64-bit integers
  because Kotlin's unsigned types cannot be called from Java;
- `MiniGrafValue::Set { value }`: a set value's elements in canonical order, in query
  results' typed forms, fact records and bind values; an element that is a set is
  `API-021`. `execute()` JSON returns a set as an array. Each language accepts its
  native set type as a plain bind value (minigraf#316);
- `MiniGrafError` variants by core error category: `Storage`, `Query`, `Parse`,
  `Api` (bad arguments, closed objects, bind mistakes) and `Other` (internal).

Add the language's idioms on top (an iterator over cursors and fact logs, a scoped
writer), and test them in the language: see `minigraf-python/tests/test_etl.py`.

## Why not depend on `minigraf-ffi`?

UniFFI's `setup_scaffolding!()` macro generates `extern "C"` symbols in the **final** cdylib
crate. Re-exporting from a crate that already called it causes duplicate symbol errors at
link time. Always embed the scaffolding in your own crate and depend on `minigraf` core
directly. See the existing binding repos (`minigraf-python`, `minigraf-java`, etc.) for
complete examples.

## License

MIT OR Apache-2.0
