# Extending Phoenix: adding a check or a report format

The most common contributions are **a new check** (another question about TTLs) and **a new report format** (for example JSON for CI). Both follow the same rule: data collection stays in `entry.rs`, decisions in `lib.rs`, presentation in `report.rs`.

## A. Adding a check

Example: `assert_extended_by(&self, before: &[TtlEntry], min_ledgers: u32)`, which asserts that a call extended every watched entry by at least `min_ledgers`.

1. **Decide the semantics and write them down first** as rustdoc on the method: what counts as passing, which entries are considered, and how temporary entries and entries created during the call are treated.
2. **Implement it on `Phoenix` in `src/lib.rs`**, built on `self.entries()` so `watch` and `include_temporary` apply automatically. Return `Result<_, SomeReport>` from a `check_…` method, and add an `assert_…` wrapper with `#[track_caller]` that panics with the report.
3. **Add a report type** in `src/report.rs` if none of the existing ones fit, and implement `Display`. Include what failed, by how much, and a fix hint. Reuse `table()` and `hint()` where possible so output stays consistent.
4. **Test it against real contracts** in `tests/phoenix.rs`. Add a small contract to the file (the existing `Forgetful` and `Careful` contracts are good starting points) and cover pass, fail, and the edge cases from your rustdoc. Test the report text for the key phrases users will rely on.
5. **Document it** in `docs/api.md` and the README's API table, and add a line to `CHANGELOG.md` under Unreleased.

## B. Adding a report format

Example: writing a JSON report to `target/phoenix-ttl/<name>.json` so CI can track TTL margins over time.

1. **Keep the crate's dependencies small.** Phoenix currently depends only on `soroban-sdk`. A new dependency such as `serde_json` should be optional, behind a cargo feature (`json = ["dep:serde_json"]`). Discuss this in the issue first.
2. **Add a pure function** in a new `src/report/…` module (or in `report.rs` if it's small) that turns a `TtlReport` or `ArchivalReport` into the format. No file I/O inside the function.
3. **Add the I/O entry point** separately (e.g. `TtlReport::write_json(&self, name: &str) -> std::io::Result<PathBuf>`), resolving `target/` with `CARGO_TARGET_DIR` and falling back to `target`.
4. **Version the format** with a `schema_version` field from the start.
5. **Test the output** with a fixed `TtlReport` built by hand (see the unit tests at the bottom of `report.rs`), so the output doesn't depend on generated addresses.

## Checks before opening a PR

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo doc -p phoenix-ttl --no-deps
```
