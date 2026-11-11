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
- `OpenOptions.allow_overrides: Option<Vec<Overridable>>` (`MaxDerivedFacts`, `MaxResults`,
  `QueryMemoryLimit`, `MaxIterations`): the query keywords query text may use to override
  settings. Absent, `overridable_defaults()` applies, the Rust `Overridable::DEFAULTS` read
  from the core library (today `MaxDerivedFacts`, `MaxResults`, `MaxIterations`); a list
  replaces it and an empty one refuses every keyword (`API-025`). Expose the defaults as a
  language constant read from `overridable_defaults()`, never written out again
  (minigraf#872);
- `OpenOptions.transaction_chunk_facts` and `OpenOptions.max_iterations` (rounds per stratum
  of a rule query, else `INT-020`; `i64::MAX` is no limit); a negative count is `API-017`;
- `MiniGrafPreparedQuery.with_max_derived_facts(n)`, `with_query_memory_limit(bytes)` and
  `with_max_iterations(n)`: each returns a new prepared query with that limit and leaves the
  receiver unchanged; `i64::MAX` is no limit, a negative value `API-017`; not gated by
  `allow_overrides`. A derived query keeps its database open, as any prepared query does:

  ```python
  reach = db.prepare("(query [:find ?x ?y :where (reach ?x ?y)])")
  small = reach.with_max_derived_facts(10_000)  # reach is unchanged
  ```
- `MiniGrafDb.register_predicate(name, MiniGrafPredicate)` and
  `register_aggregate(name, MiniGrafAggregate)`: user functions written in the language
  (minigraf#180), described in the next section.

Add the language's idioms on top (an iterator over cursors and fact logs, a scoped
writer), and test them in the language: see `minigraf-python/tests/test_etl.py`.

## User functions

Specified in minigraf's
[`docs/superpowers/specs/2026-10-24-udf-uniffi-design.md`](https://github.com/project-minigraf/minigraf/blob/v3/docs/superpowers/specs/2026-10-24-udf-uniffi-design.md)
§6. Copy this section into the binding's README with the language's own example.

- A predicate implements `MiniGrafPredicate.test(value) -> bool` and is called in Datalog
  as `[(name ?v)]`. An aggregate implements `MiniGrafAggregate.accumulator()`, which
  returns a new `MiniGrafAccumulator` (`step(value)`, `finish(count)`) for each group or
  window partition, and is used as `(name ?v)` in `:find` or with `:over`.
- **Failures.** A callback that throws stops the query with `QRY-014`, raised as
  `MiniGrafError.Query`; it is never read as `false` or a missing value. Throw
  `MiniGrafUdfError.Failed(msg)` to choose the message. Any other exception also fails the
  call, with its first line (class and message) as the message. A `Ref` from `finish`
  whose value is not a UUID is a failing call too.
- **Lifecycle.** The database keeps a strong reference to each registered object until
  the database and every cursor and prepared query opened from it are dropped. There is
  no unregister. Predicates and aggregates have separate names, and registering a name
  twice in either is an error (`INT-040`). A query sees the functions
  registered when it opened.
- **Threading.** A callback runs on the thread that calls `execute` (of the database or
  a prepared query) or a cursor's `next_batch`, including a prepared query's cursor. Queries on different threads may call the same object at
  once, so it must be thread-safe. One accumulator is used by one query on one thread at
  a time.
- **Cost.** Each call crosses the FFI once: one foreign call per value.
- **Re-entry.** A callback may query the same database, open a cursor or register another
  function. Inside a write transaction, the calls the core refuses there still return
  `INT-001`. No binding lock is held while the core runs: each `MiniGrafDb` method locks
  its handle only to clone the database.

What each generator produces (UniFFI 0.31.1; the traits need `#[uniffi::export(with_foreign)]`,
without which they are not implementable outside Rust):

| Language | Implement | Throwing |
|---|---|---|
| Python | subclass `MiniGrafPredicate` / `MiniGrafAggregate` / `MiniGrafAccumulator` | raise `MiniGrafUdfError.Failed(msg)`, or any exception. UniFFI's ctypes calls release the GIL; the callback re-acquires it |
| Kotlin | `interface MiniGrafPredicate` etc. (`MiniGrafPredicateImpl` is the Rust-side class) | the interfaces carry no `@Throws`: Kotlin throws `MiniGrafUdfException.Failed(msg)`; Java, which cannot throw that checked exception from them, throws an unchecked exception instead |
| Swift | `protocol MiniGrafPredicate: AnyObject, Sendable` etc. | the methods are `throws`: throw `MiniGrafUdfError.Failed(msg:)`. An implementation is a class that is `Sendable` (immutable `final`, or `@unchecked Sendable` with its own locking) |

## Write transactions

`MiniGrafDb.begin_write()` / `begin_write_with(TxOptions { synchronous, chunk_facts })`
return a `MiniGrafWriteTransaction` with `execute`, `commit`, `rollback` and `is_open`,
built on core's `OwnedWriteTransaction` (minigraf#824; first in the release built on
core v3.0.0, core commit `CORE_COMMIT`). Its behaviour, which every binding documents:

- `execute` returns what `MiniGrafDb.execute` returns; a staged write returns
  `{"ok":true}`, a query reads the staged state, a failed statement stages nothing and
  leaves the transaction open. No cursors or prepared queries inside a transaction.
- `commit` applies everything as one transaction (a failed commit rolls back);
  `rollback` discards it. A finished transaction's `execute` and `commit` are API-018;
  `rollback` does nothing. Freed while open, it rolls back. Commit is always explicit:
  a language's scope form (Python `with`, Kotlin `use`, a Swift closure) rolls back on
  exit unless committed.
- While open it holds the write lock: on the thread that began it, and inside its own
  statements' user functions, database calls are INT-001; other threads' writes and
  begins wait, their queries read the committed state. It keeps its database open, and
  rules registered in it survive a rollback.
- Kotlin: the generated `close()` frees the object (and so rolls back unless
  committed); add `beginWrite(options)` over `beginWriteWith`.

## Why not depend on `minigraf-ffi`?

UniFFI's `setup_scaffolding!()` macro generates `extern "C"` symbols in the **final** cdylib
crate. Re-exporting from a crate that already called it causes duplicate symbol errors at
link time. Always embed the scaffolding in your own crate and depend on `minigraf` core
directly. See the existing binding repos (`minigraf-python`, `minigraf-java`, etc.) for
complete examples.

## License

MIT OR Apache-2.0
