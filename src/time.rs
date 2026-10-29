//! Converting between ledgers and wall-clock time.
//!
//! Stellar closes a ledger roughly every 5 seconds, but the real interval
//! varies. These helpers use a fixed estimate so tests are deterministic;
//! treat the results as approximations, not guarantees.

/// The ledger close time assumed by the helpers in this module, in seconds.
pub const SECONDS_PER_LEDGER: u64 = 5;

/// Ledgers in `n` hours at [`SECONDS_PER_LEDGER`].
pub const fn hours(n: u32) -> u32 {
    (n as u64 * 3_600 / SECONDS_PER_LEDGER) as u32
}

/// Ledgers in `n` days at [`SECONDS_PER_LEDGER`] (17,280 per day).
pub const fn days(n: u32) -> u32 {
    (n as u64 * 86_400 / SECONDS_PER_LEDGER) as u32
}

/// Ledgers in `n` weeks at [`SECONDS_PER_LEDGER`].
pub const fn weeks(n: u32) -> u32 {
    days(n * 7)
}

/// Formats a number of ledgers as an approximate duration, e.g. `≈ 5.7 days`.
pub fn describe(ledgers: u64, seconds_per_ledger: u64) -> String {
    let secs = ledgers * seconds_per_ledger;
    let (value, unit) = if secs >= 86_400 {
        (secs as f64 / 86_400.0, "days")
    } else if secs >= 3_600 {
        (secs as f64 / 3_600.0, "hours")
    } else {
        (secs as f64 / 60.0, "minutes")
    };
    format!("≈ {value:.1} {unit}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts() {
        assert_eq!(days(1), 17_280);
        assert_eq!(hours(1), 720);
        assert_eq!(weeks(1), 120_960);
        assert_eq!(describe(4_096, 5), "≈ 5.7 hours");
        assert_eq!(describe(518_400, 5), "≈ 30.0 days");
        assert_eq!(describe(12, 5), "≈ 1.0 minutes");
    }
}
