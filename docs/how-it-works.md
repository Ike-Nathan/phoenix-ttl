# How it works

## 1. TTL rules Phoenix relies on

From the [Stellar state-archival docs](https://developers.stellar.org/docs/learn/fundamentals/contract-development/storage/state-archival) and soroban-sdk 29.0.1:

- Each contract instance, Wasm code entry and contract data entry has a `live_until` ledger. It is live up to and including that ledger.
- **Persistent and instance** entries are **archived** after that. Instance storage lives inside the instance entry and shares its TTL. Archived entries must be restored before use. Since protocol 23, a transaction can restore them automatically if they are in its restore list (simulation fills this in). "Normal rent and fees still apply", and a transaction that touches an archived entry not in that list fails.
- **Contract code** is also archived, and a contract call needs **both** the instance and the code to be live.
- **Temporary** entries are **deleted** when they expire and cannot be restored.
- `env.storage().instance().extend_ttl(threshold, extend_to)` extends the current contract's **instance and code** together (`extend_current_contract_instance_and_code_ttl` in the SDK).
- `extend_ttl(key, threshold, extend_to)` on persistent or temporary storage extends one entry to `current + extend_to`. The SDK docs say this happens when the remaining TTL is _below_ `threshold`. In the soroban-sdk 29.0.1 test host we observed it also happening when the remaining TTL is _exactly_ `threshold` (an entry live until day 60, touched on day 30 with a 30-day threshold, was extended). Don't write tests that depend on the exact boundary.

## 2. Why unit tests miss it

Before protocol 23, the SDK made tests fail when they touched an archived entry. The [SDK 23 migration note](https://docs.rs/soroban-sdk/latest/soroban_sdk/_migrating/v23_archived_testing/index.html) explains that tests now **emulate automatic restoration** instead. Reading an archived entry in a test returns the old value. We confirmed this with soroban-sdk 29.0.1: a persistent entry live until ledger 4,095 is still readable at ledger 5,000.

The migration note recommends checking TTLs explicitly. Phoenix turns that into a whole-contract check.

## 3. Collecting entries

`collect(&env)` calls `env.to_ledger_snapshot()`, which returns every ledger entry in the test host with its `live_until`. For each entry:

| Ledger entry | Becomes | Owner |
| --- | --- | --- |
| Contract data with key `LedgerKeyContractInstance` | `Instance` | that contract |
| Contract data, persistent | `Persistent` | that contract |
| Contract data, temporary | `Temporary` | that contract |
| Contract data with key `LedgerKeyNonce` | skipped (host bookkeeping) | |
| Contract code | `Code` | every contract whose instance's executable is `Wasm(hash)` with that hash |
| Accounts, trustlines (no TTL) | skipped | |

Entries are sorted soonest-expiring first, so reports lead with the most urgent.

## 4. Checking "live for N ledgers"

At current ledger `c`, an entry with `live_until = u` has `u − c` ledgers left and is archived at ledger `u + 1`. `check_live_for(n)` fails for entries with `u − c < n`, or that have already expired (`u < c`).

## 5. Simulating usage

The SDK test host restores archived entries on access, so Phoenix must catch expiry **before** the ledger moves past it. Each step:

```text
check: any watched entry with live_until < now + every?  → ArchivalReport(step)
advance: sequence += every; timestamp += every × seconds_per_ledger
run: step(&env, n)
```

Example from the vault: entries are extended to 60 days on write and checked every 30 days. The balance of a user who deposits once survives steps 1 and 2 (`live_until` = day 60 = `now + every` at step 2), and is reported at step 3.

## 6. Reports

`TtlReport` and `ArchivalReport` implement `Display`. The output is a table of owner, kind, key, `live_until` and when the entry is archived or deleted, followed by fix hints for the kinds of entries listed (instance/code, persistent or temporary), and a link to the archival docs. Assertions panic with this text, so it appears directly in `cargo test` output.
