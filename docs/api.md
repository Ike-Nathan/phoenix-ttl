# API and configuration reference

Phoenix is configured in code. There are no config files or environment variables. Rustdoc (`cargo doc --open -p phoenix-ttl`) is the authoritative reference; this page summarises it.

## Installing

```toml
[dev-dependencies]
phoenix-ttl = { git = "https://github.com/Ike-Nathan/phoenix-ttl", tag = "v0.1.0" }
soroban-sdk = { version = "29", features = ["testutils"] }
```

Supported: soroban-sdk 29.x, Rust 1.91 or later.

## `Phoenix`

```rust
let phoenix = Phoenix::new(&env)
    .watch(&vault_id)           // repeatable; default: all contracts
    .include_temporary(false)   // default
    .seconds_per_ledger(5);     // default
```

| Method | Returns | Notes |
| --- | --- | --- |
| `new(&Env)` | `Phoenix` | Cheap. Clone it to make variants. |
| `watch(&Address)` | `Self` | Keeps entries whose owner is this contract. Code entries are kept when a watched contract's instance uses that Wasm. |
| `include_temporary(bool)` | `Self` | Temporary entries are deleted (not archived) on expiry. |
| `seconds_per_ledger(u64)` | `Self` | Used for "≈" durations and for advancing the timestamp in simulations. Minimum 1. |
| `entries()` | `Vec<TtlEntry>` | Watched entries sorted by `live_until`, then kind, owner and key. |
| `report()` | `TtlReport` | All watched entries. Implements `Display`. |
| `check_live_for(ledgers: u32)` | `Result<(), TtlReport>` | Fails if any watched entry has fewer than `ledgers` ledgers left after the current one. The error lists only the failing entries. |
| `assert_live_for(ledgers: u32)` | `()` | Panics with the report. |
| `simulate(every: u32, steps: u32, f)` | `Result<Survival, ArchivalReport>` | See below. `every` must be > 0. |
| `assert_survives(every, steps, f)` | `Survival` | Panics with the report. |

### Simulation semantics

For `n` in `1..=steps`:

1. `now` = current ledger sequence, `next` = `now + every`.
2. If any watched entry has `live_until < next`, return `ArchivalReport { step: n, from: now, to: next, entries }`. Those entries would be archived (or deleted) before the step runs.
3. Set the ledger sequence to `next`, and add `every × seconds_per_ledger` to the ledger timestamp.
4. Call `f(&env, n)`.

Entries created by a step are checked from the next step on. `Survival::closest_call` holds the smallest margin seen before any step, and the entry it belonged to.

## Types

| Type | Fields |
| --- | --- |
| `TtlEntry` | `kind: EntryKind`, `contracts: Vec<String>` (strkeys), `key: String` (rendered), `live_until: u32`. `remaining(current) -> Option<u32>`. |
| `EntryKind` | `Instance`, `Persistent`, `Temporary`, `Code`. |
| `TtlReport` | `current`, `required: Option<u32>`, `entries`, `seconds_per_ledger`. |
| `ArchivalReport` | `step`, `from`, `to`, `entries`, `seconds_per_ledger`. |
| `Survival` | `steps`, `final_ledger`, `closest_call: Option<(u32, TtlEntry)>`. |

## Helpers

| Function | Value |
| --- | --- |
| `SECONDS_PER_LEDGER` | `5` |
| `hours(n)` | `n × 720` ledgers |
| `days(n)` | `n × 17_280` ledgers |
| `weeks(n)` | `n × 120_960` ledgers |
| `collect(&Env)` | Every TTL-bearing entry, unfiltered (including temporary). |
| `render::key`, `render::value`, `render::short_address` | The renderers used in reports. |

## Test-environment settings that matter

These are `soroban_sdk::testutils::Ledger` setters, not Phoenix options, but they change what Phoenix sees:

| Setting | Test default | Why it matters |
| --- | --- | --- |
| `set_min_persistent_entry_ttl` | 4,096 | New persistent and instance entries start with this TTL. |
| `set_min_temp_entry_ttl` | 16 | New temporary entries start with this TTL. |
| `set_max_entry_ttl` | 6,312,000 | `extend_ttl` is capped here. |

Real networks use different values. Check the [resource limits page](https://developers.stellar.org/docs/networks/resource-limits-fees) and set them if your policy depends on them.
