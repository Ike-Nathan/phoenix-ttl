//! Reading every TTL-bearing ledger entry out of a test `Env`.

use std::collections::BTreeMap;
use std::fmt;

use soroban_sdk::xdr::{ContractDataDurability, ContractExecutable, LedgerEntryData, ScVal};
use soroban_sdk::Env;

use crate::render;

/// What kind of TTL-bearing entry this is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EntryKind {
    /// The contract instance entry. Instance storage lives inside it and
    /// shares its TTL.
    Instance,
    /// A persistent storage entry.
    Persistent,
    /// A temporary storage entry. These are deleted, not archived, when they
    /// expire, and are ignored by default.
    Temporary,
    /// The Wasm code entry used by one or more contract instances.
    Code,
}

impl fmt::Display for EntryKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            EntryKind::Instance => "instance",
            EntryKind::Persistent => "persistent",
            EntryKind::Temporary => "temporary",
            EntryKind::Code => "code",
        })
    }
}

/// One ledger entry with a TTL, as seen in the test environment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TtlEntry {
    pub kind: EntryKind,
    /// Contracts that own (or, for code entries, use) this entry, as strkeys.
    pub contracts: Vec<String>,
    /// Human-readable key, e.g. `Shares(CAAA…K3IM)`, `contract instance`, `wasm 74beb3d1…`.
    pub key: String,
    /// Last ledger on which the entry is live. It is archived (or deleted, if
    /// temporary) from `live_until + 1`.
    pub live_until: u32,
}

impl TtlEntry {
    /// Ledgers left after `current`, or `None` if the entry has already expired.
    pub fn remaining(&self, current: u32) -> Option<u32> {
        self.live_until.checked_sub(current)
    }

    /// Short owner label for reports: `CAAA…ITA4`, or `CAAA…ITA4 +1` for shared code.
    pub fn owner_label(&self) -> String {
        match self.contracts.as_slice() {
            [] => "(unused)".into(),
            [one] => render::short_address(one),
            [first, rest @ ..] => format!("{} +{}", render::short_address(first), rest.len()),
        }
    }
}

/// Collects every contract instance, contract data and contract code entry
/// from the environment's current ledger state, sorted by `live_until`.
///
/// Auth nonce entries are skipped: they are host bookkeeping, not contract state.
pub fn collect(env: &Env) -> Vec<TtlEntry> {
    let snapshot = env.to_ledger_snapshot();
    let mut out = Vec::new();
    // wasm hash -> contracts whose instance uses it
    let mut code_users: BTreeMap<[u8; 32], Vec<String>> = BTreeMap::new();
    let mut code_entries: Vec<([u8; 32], u32)> = Vec::new();

    for (_key, (entry, live_until)) in &snapshot.ledger_entries {
        let Some(live_until) = *live_until else {
            continue; // accounts, trustlines: no TTL
        };
        match &entry.data {
            LedgerEntryData::ContractData(d) => {
                let contract = render::address(&d.contract);
                match (&d.key, &d.val) {
                    (ScVal::LedgerKeyContractInstance, ScVal::ContractInstance(inst)) => {
                        if let ContractExecutable::Wasm(hash) = &inst.executable {
                            code_users.entry(hash.0).or_default().push(contract.clone());
                        }
                        out.push(TtlEntry {
                            kind: EntryKind::Instance,
                            contracts: vec![contract],
                            key: "contract instance".into(),
                            live_until,
                        });
                    }
                    (ScVal::LedgerKeyNonce(_), _) => {}
                    (key, _) => out.push(TtlEntry {
                        kind: match d.durability {
                            ContractDataDurability::Persistent => EntryKind::Persistent,
                            ContractDataDurability::Temporary => EntryKind::Temporary,
                        },
                        contracts: vec![contract],
                        key: render::key(key),
                        live_until,
                    }),
                }
            }
            LedgerEntryData::ContractCode(c) => code_entries.push((c.hash.0, live_until)),
            _ => {}
        }
    }

    for (hash, live_until) in code_entries {
        let mut contracts = code_users.remove(&hash).unwrap_or_default();
        contracts.sort();
        out.push(TtlEntry {
            kind: EntryKind::Code,
            contracts,
            key: format!("wasm {}…", &render::hex(&hash)[..8]),
            live_until,
        });
    }

    out.sort_by(|a, b| {
        (a.live_until, a.kind, &a.contracts, &a.key).cmp(&(
            b.live_until,
            b.kind,
            &b.contracts,
            &b.key,
        ))
    });
    out
}
