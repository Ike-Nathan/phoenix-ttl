//! Functional tests plus the TTL policy, written with phoenix-ttl.
extern crate std;

use super::*;
use phoenix_ttl::{days, EntryKind, Phoenix};
use soroban_sdk::testutils::{Address as _, EnvTestConfig};
use soroban_sdk::token::StellarAssetClient;

struct Setup<'a> {
    env: Env,
    id: Address,
    vault: VaultClient<'a>,
    sac: StellarAssetClient<'a>,
}

fn setup<'a>() -> Setup<'a> {
    // Simulations advance the ledger by a year; skip writing test snapshots.
    let env = Env::new_with_config(EnvTestConfig {
        capture_snapshot_at_drop: false,
    });
    env.mock_all_auths();
    let issuer = Address::generate(&env);
    let token = env.register_stellar_asset_contract_v2(issuer);
    let id = env.register(Vault, (&token.address(),));
    Setup {
        vault: VaultClient::new(&env, &id),
        sac: StellarAssetClient::new(&env, &token.address()),
        id,
        env,
    }
}

impl Setup<'_> {
    fn user(&self, balance: i128) -> Address {
        let u = Address::generate(&self.env);
        self.sac.mint(&u, &balance);
        u
    }
}

#[test]
fn deposit_and_withdraw() {
    let s = setup();
    let u = s.user(1_000);
    assert_eq!(s.vault.deposit(&u, &400), 400);
    assert_eq!(s.vault.withdraw(&u, &150), 250);
    assert_eq!(
        s.vault.try_withdraw(&u, &251),
        Err(Ok(VaultError::InsufficientShares))
    );
}

/// After any deposit, everything the vault owns is live for at least 30 days.
#[test]
fn state_is_live_for_30_days_after_a_deposit() {
    let s = setup();
    let u = s.user(1_000);
    s.vault.deposit(&u, &100);
    Phoenix::new(&s.env).watch(&s.id).assert_live_for(days(30));
}

/// A user who deposits every 30 days keeps the vault and their balance
/// alive for a year.
#[test]
fn monthly_user_survives_a_year() {
    let s = setup();
    let u = s.user(1_000);
    s.vault.deposit(&u, &100);
    let survival = Phoenix::new(&s.env)
        .watch(&s.id)
        .assert_survives(days(30), 12, |_, _| {
            s.vault.deposit(&u, &1);
        });
    std::println!("{survival}");
}

/// The documented trade-off: an idle user's balance is archived after
/// about 60 days even while the vault itself stays live.
#[test]
fn idle_balance_is_archived_after_60_days() {
    let s = setup();
    let idle = s.user(1_000);
    let active = s.user(1_000);
    s.vault.deposit(&idle, &100);

    let err = Phoenix::new(&s.env)
        .watch(&s.id)
        .simulate(days(30), 12, |_, _| {
            s.vault.deposit(&active, &1);
        })
        .unwrap_err();
    std::println!("{err}");
    assert_eq!(err.step, 3, "archived between day 60 and day 90");
    assert_eq!(err.entries.len(), 1);
    assert_eq!(err.entries[0].kind, EntryKind::Persistent);
    assert!(err.entries[0].key.starts_with("Shares("));
}
