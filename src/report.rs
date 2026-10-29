//! Human-readable reports. These are what users see when a test fails, so
//! they say what will happen, when, and what to do about it.

use std::fmt;

use crate::entry::{EntryKind, TtlEntry};
use crate::time::describe;

/// Watched entries and their remaining TTL. Returned by
/// [`Phoenix::report`](crate::Phoenix::report) and, listing only the
/// failing entries, by [`Phoenix::check_live_for`](crate::Phoenix::check_live_for).
#[derive(Clone, Debug)]
pub struct TtlReport {
    /// Ledger sequence when the report was made.
    pub current: u32,
    /// Required remaining TTL, when this report is a failed check.
    pub required: Option<u32>,
    pub entries: Vec<TtlEntry>,
    pub seconds_per_ledger: u64,
}

/// The first point in a [`Phoenix::simulate`](crate::Phoenix::simulate) run
/// at which watched state would be archived or deleted.
#[derive(Clone, Debug)]
pub struct ArchivalReport {
    /// 1-based step that could not run safely.
    pub step: u32,
    /// Ledger before the step.
    pub from: u32,
    /// Ledger the step would have run at.
    pub to: u32,
    /// Entries whose TTL ends before `to`.
    pub entries: Vec<TtlEntry>,
    pub seconds_per_ledger: u64,
}

/// Result of a successful simulation.
#[derive(Clone, Debug)]
pub struct Survival {
    pub steps: u32,
    pub final_ledger: u32,
    /// The smallest TTL margin seen before any step, and the entry it belonged
    /// to. Useful to see how close a contract came to losing state.
    pub closest_call: Option<(u32, TtlEntry)>,
}

impl fmt::Display for TtlReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.required {
            Some(req) => writeln!(
                f,
                "phoenix-ttl: {} entr{} will not stay live for {} ledgers ({}) from ledger {}:",
                self.entries.len(),
                if self.entries.len() == 1 { "y" } else { "ies" },
                req,
                describe(u64::from(req), self.seconds_per_ledger),
                self.current
            )?,
            None => writeln!(
                f,
                "phoenix-ttl: {} entries at ledger {}:",
                self.entries.len(),
                self.current
            )?,
        }
        table(f, &self.entries, self.current, self.seconds_per_ledger)?;
        if self.required.is_some() {
            hint(f, &self.entries)?;
        }
        Ok(())
    }
}

impl fmt::Display for ArchivalReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "phoenix-ttl: state would be lost before step {} (ledger {} → {}, {} later):",
            self.step,
            self.from,
            self.to,
            describe(u64::from(self.to - self.from), self.seconds_per_ledger)
        )?;
        table(f, &self.entries, self.from, self.seconds_per_ledger)?;
        hint(f, &self.entries)
    }
}

impl fmt::Display for Survival {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "survived {} steps, ending at ledger {}",
            self.steps, self.final_ledger
        )?;
        if let Some((margin, e)) = &self.closest_call {
            write!(
                f,
                "; closest call: {} `{}` had {} ledgers left",
                e.kind, e.key, margin
            )?;
        }
        Ok(())
    }
}

fn table(f: &mut fmt::Formatter<'_>, entries: &[TtlEntry], current: u32, spl: u64) -> fmt::Result {
    let rows: Vec<[String; 5]> = entries
        .iter()
        .map(|e| {
            let lost = if e.kind == EntryKind::Temporary {
                "deleted"
            } else {
                "archived"
            };
            let when = match e.remaining(current) {
                Some(r) => format!(
                    "{lost} at ledger {} (in {} ledgers, {})",
                    u64::from(e.live_until) + 1,
                    u64::from(r) + 1,
                    describe(u64::from(r) + 1, spl)
                ),
                None => format!(
                    "already {lost} since ledger {}",
                    u64::from(e.live_until) + 1
                ),
            };
            [
                e.owner_label(),
                e.kind.to_string(),
                e.key.clone(),
                format!("live until {}", e.live_until),
                when,
            ]
        })
        .collect();
    let mut widths = [0usize; 4];
    for r in &rows {
        for (w, cell) in widths.iter_mut().zip(r.iter()) {
            *w = (*w).max(cell.chars().count());
        }
    }
    for r in &rows {
        writeln!(
            f,
            "  {:<w0$}  {:<w1$}  {:<w2$}  {:<w3$}  {}",
            r[0],
            r[1],
            r[2],
            r[3],
            r[4],
            w0 = widths[0],
            w1 = widths[1],
            w2 = widths[2],
            w3 = widths[3]
        )?;
    }
    Ok(())
}

fn hint(f: &mut fmt::Formatter<'_>, entries: &[TtlEntry]) -> fmt::Result {
    let has = |k: EntryKind| entries.iter().any(|e| e.kind == k);
    writeln!(f, "How to fix:")?;
    if has(EntryKind::Instance) || has(EntryKind::Code) {
        writeln!(
            f,
            "  - instance/code: call env.storage().instance().extend_ttl(threshold, extend_to) in your entry points (it extends both)."
        )?;
    }
    if has(EntryKind::Persistent) {
        writeln!(
            f,
            "  - persistent: call env.storage().persistent().extend_ttl(&key, threshold, extend_to) when the entry is written or read."
        )?;
    }
    if has(EntryKind::Temporary) {
        writeln!(f, "  - temporary: expiry deletes the entry for good; use persistent storage if it must survive.")?;
    }
    writeln!(
        f,
        "  On a real network, archived entries must be restored (paying fees) before use; see https://developers.stellar.org/docs/learn/fundamentals/contract-development/storage/state-archival"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(kind: EntryKind, live_until: u32) -> TtlEntry {
        TtlEntry {
            kind,
            contracts: vec!["CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAITA4".into()],
            key: "Shares(CAAA…K3IM)".into(),
            live_until,
        }
    }

    #[test]
    fn counts_ledgers_up_to_the_archival_ledger() {
        let r = TtlReport {
            current: 0,
            required: Some(100),
            entries: vec![entry(EntryKind::Persistent, 10)],
            seconds_per_ledger: 5,
        };
        let text = r.to_string();
        assert!(
            text.contains("1 entry will not stay live for 100 ledgers"),
            "{text}"
        );
        assert!(
            text.contains("archived at ledger 11 (in 11 ledgers, ≈ 0.9 minutes)"),
            "{text}"
        );
        assert!(
            text.contains("CAAA…ITA4  persistent  Shares(CAAA…K3IM)"),
            "{text}"
        );
        assert!(text.contains("persistent().extend_ttl"), "{text}");
        assert!(!text.contains("instance().extend_ttl"), "{text}");
    }

    #[test]
    fn temporary_entries_are_deleted_not_archived() {
        let r = TtlReport {
            current: 50,
            required: Some(1),
            entries: vec![entry(EntryKind::Temporary, 20)],
            seconds_per_ledger: 5,
        };
        let text = r.to_string();
        assert!(text.contains("already deleted since ledger 21"), "{text}");
        assert!(text.contains("expiry deletes the entry for good"), "{text}");
    }
}
