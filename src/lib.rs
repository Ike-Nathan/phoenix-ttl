//! # phoenix-ttl
//!
//! TTL-safety tests for Soroban contracts.
//!
//! Every Soroban ledger entry has a time to live. When a persistent entry,
//! a contract instance or its Wasm code runs out of TTL it is **archived**
//! and must be restored (with fees) before it can be used again; temporary
//! entries are **deleted**. Since protocol 23 the SDK's test environment
//! restores archived entries automatically, so a contract that never extends
//! its TTLs still passes every unit test. `phoenix-ttl` makes that visible.
//!
//! ```
//! # mod counter {
//! #     use soroban_sdk::{contract, contractimpl, symbol_short, Env};
//! #     #[contract]
//! #     pub struct Counter;
//! #     #[contractimpl]
//! #     impl Counter {
//! #         pub fn bump(env: Env) {
//! #             let k = symbol_short!("n");
//! #             let n: u32 = env.storage().persistent().get(&k).unwrap_or(0);
//! #             env.storage().persistent().set(&k, &(n + 1));
//! #             env.storage().persistent().extend_ttl(&k, phoenix_ttl::days(7), phoenix_ttl::days(30));
//! #             env.storage().instance().extend_ttl(phoenix_ttl::days(7), phoenix_ttl::days(30));
//! #         }
//! #     }
//! # }
//! # use counter::{Counter, CounterClient};
//! use phoenix_ttl::{days, Phoenix};
//! use soroban_sdk::Env;
//!
//! let env = Env::default();
//! let id = env.register(Counter, ());
//! let client = CounterClient::new(&env, &id);
//! client.bump();
//!
//! let phoenix = Phoenix::new(&env).watch(&id);
//! // Everything the contract owns must stay live for at least a week…
//! phoenix.assert_live_for(days(7));
//! // …and keep living if someone calls `bump` once a week for a year.
//! phoenix.assert_survives(days(7), 52, |_env, _step| client.bump());
//! ```

mod entry;
pub mod render;
mod report;
pub mod time;

use soroban_sdk::testutils::Ledger;
use soroban_sdk::xdr::ScAddress;
use soroban_sdk::{Address, Env};

pub use entry::{collect, EntryKind, TtlEntry};
pub use report::{ArchivalReport, Survival, TtlReport};
pub use time::{days, hours, weeks, SECONDS_PER_LEDGER};

/// Checks TTLs of ledger entries in a Soroban test [`Env`].
///
/// By default every contract in the environment is watched. Use
/// [`Phoenix::watch`] to focus on the contracts you own (recommended: token
/// contracts used in tests manage their own TTLs).
#[derive(Clone)]
pub struct Phoenix<'a> {
    env: &'a Env,
    watched: Vec<String>,
    include_temporary: bool,
    seconds_per_ledger: u64,
}

impl<'a> Phoenix<'a> {
    pub fn new(env: &'a Env) -> Self {
        Self {
            env,
            watched: Vec::new(),
            include_temporary: false,
            seconds_per_ledger: SECONDS_PER_LEDGER,
        }
    }

    /// Only check entries owned by `contract` (call repeatedly to watch several).
    /// Code entries are included when a watched contract's instance uses them.
    pub fn watch(mut self, contract: &Address) -> Self {
        self.watched.push(ScAddress::from(contract).to_string());
        self
    }

    /// Also check temporary entries. Off by default, because temporary
    /// entries are designed to expire.
    pub fn include_temporary(mut self, yes: bool) -> Self {
        self.include_temporary = yes;
        self
    }

    /// Ledger close time used for time estimates in reports and for advancing
    /// the ledger timestamp in [`Phoenix::simulate`]. Defaults to 5 seconds.
    pub fn seconds_per_ledger(mut self, seconds: u64) -> Self {
        self.seconds_per_ledger = seconds.max(1);
        self
    }

    /// The watched entries, soonest-expiring first.
    pub fn entries(&self) -> Vec<TtlEntry> {
        collect(self.env)
            .into_iter()
            .filter(|e| self.include_temporary || e.kind != EntryKind::Temporary)
            .filter(|e| {
                self.watched.is_empty() || e.contracts.iter().any(|c| self.watched.contains(c))
            })
            .collect()
    }

    /// A report of all watched entries and their remaining TTL, for printing.
    pub fn report(&self) -> TtlReport {
        TtlReport {
            current: self.env.ledger().sequence(),
            required: None,
            entries: self.entries(),
            seconds_per_ledger: self.seconds_per_ledger,
        }
    }

    /// Checks that every watched entry stays live for at least `ledgers` more
    /// ledgers. On failure, the report lists only the entries that fall short.
    pub fn check_live_for(&self, ledgers: u32) -> Result<(), TtlReport> {
        let current = self.env.ledger().sequence();
        let short: Vec<TtlEntry> = self
            .entries()
            .into_iter()
            .filter(|e| e.remaining(current).is_none_or(|r| r < ledgers))
            .collect();
        if short.is_empty() {
            Ok(())
        } else {
            Err(TtlReport {
                current,
                required: Some(ledgers),
                entries: short,
                seconds_per_ledger: self.seconds_per_ledger,
            })
        }
    }

    /// Like [`Phoenix::check_live_for`], but panics with a readable report.
    #[track_caller]
    pub fn assert_live_for(&self, ledgers: u32) {
        if let Err(report) = self.check_live_for(ledgers) {
            panic!("{report}");
        }
    }

    /// Simulates the contract being used over time.
    ///
    /// Repeats `steps` times: check that no watched entry would expire within
    /// the next `every` ledgers, advance the ledger sequence by `every` (and
    /// the timestamp by `every × seconds_per_ledger`), then call `step(env, n)`
    /// with the 1-based step number. Entries created by earlier steps are
    /// checked in later ones.
    ///
    /// Returns the first set of entries that would be archived or deleted
    /// before the next step, so the failure points at the exact moment state
    /// would be lost on a real network.
    pub fn simulate<F>(
        &self,
        every: u32,
        steps: u32,
        mut step: F,
    ) -> Result<Survival, ArchivalReport>
    where
        F: FnMut(&Env, u32),
    {
        assert!(
            every > 0,
            "phoenix-ttl: `every` must be at least one ledger"
        );
        let mut lowest: Option<(u32, TtlEntry)> = None;
        for n in 1..=steps {
            let now = self.env.ledger().sequence();
            let next = now
                .checked_add(every)
                .expect("phoenix-ttl: ledger sequence overflowed u32");
            let entries = self.entries();
            let expiring: Vec<TtlEntry> = entries
                .iter()
                .filter(|e| e.live_until < next)
                .cloned()
                .collect();
            if !expiring.is_empty() {
                return Err(ArchivalReport {
                    step: n,
                    from: now,
                    to: next,
                    entries: expiring,
                    seconds_per_ledger: self.seconds_per_ledger,
                });
            }
            for e in entries {
                let margin = e.live_until - now;
                if lowest.as_ref().is_none_or(|(m, _)| margin < *m) {
                    lowest = Some((margin, e));
                }
            }
            let secs = u64::from(every) * self.seconds_per_ledger;
            self.env.ledger().with_mut(|li| {
                li.sequence_number = next;
                li.timestamp = li.timestamp.saturating_add(secs);
            });
            step(self.env, n);
        }
        Ok(Survival {
            steps,
            final_ledger: self.env.ledger().sequence(),
            closest_call: lowest,
        })
    }

    /// Like [`Phoenix::simulate`], but panics with a readable report.
    #[track_caller]
    pub fn assert_survives<F>(&self, every: u32, steps: u32, step: F) -> Survival
    where
        F: FnMut(&Env, u32),
    {
        match self.simulate(every, steps, step) {
            Ok(s) => s,
            Err(report) => panic!("{report}"),
        }
    }
}
