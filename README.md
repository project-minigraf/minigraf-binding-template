# minigraf-binding-template

Template repository for building a new [Minigraf](https://github.com/project-minigraf/minigraf) language binding.

## What's in this repo

| File | Purpose |
|---|---|
| `Cargo.toml` | Rust shim crate — depends on `minigraf` core, produces a cdylib |
| `src/lib.rs` | UniFFI scaffolding with comments at every extension point |
| `src/uniffi_bindgen.rs` | Entry point for the `uniffi-bindgen` CLI binary |
| `.github/workflows/ci.yml` | Starter CI — runs `cargo test` |

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
6. Add a `release.yml` that receives the `core-release` repository_dispatch event from
   the minigraf cascade, pins the new `minigraf` version in `Cargo.toml`, commits, tags,
   and publishes your artifact.

## Why not depend on `minigraf-ffi`?

UniFFI's `setup_scaffolding!()` macro generates `extern "C"` symbols in the **final** cdylib
crate. Re-exporting from a crate that already called it causes duplicate symbol errors at
link time. Always embed the scaffolding in your own crate and depend on `minigraf` core
directly. See the existing binding repos (`minigraf-python`, `minigraf-java`, etc.) for
complete examples.

## License

MIT OR Apache-2.0
