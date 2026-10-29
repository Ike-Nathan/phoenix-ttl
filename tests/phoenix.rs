//! End-to-end tests against real contracts registered in a test `Env`.

use phoenix_ttl::{days, EntryKind, Phoenix};
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

#[contracttype]
pub enum Key {
    Balance(Address),
    Session(Address),
}

/// Writes balances but never extends any TTL.
#[contract]
pub struct Forgetful;

#[contractimpl]
impl Forgetful {
    pub fn deposit(env: Env, user: Address, amount: i128) {
        let k = Key::Balance(user);
        let b: i128 = env.storage().persistent().get(&k).unwrap_or(0);
        env.storage().persistent().set(&k, &(b + amount));
    }
}

/// Extends the balance and the instance on every write, and keeps a
/// short-lived temporary session.
#[contract]
pub struct Careful;

#[contractimpl]
impl Careful {
    pub fn deposit(env: Env, user: Address, amount: i128) {
        let k = Key::Balance(user.clone());
        let b: i128 = env.storage().persistent().get(&k).unwrap_or(0);
        env.storage().persistent().set(&k, &(b + amount));
        env.storage()
            .persistent()
            .extend_ttl(&k, days(30), days(60));
        env.storage().instance().extend_ttl(days(30), days(60));
        env.storage().temporary().set(&Key::Session(user), &true);
    }
}

fn setup_forgetful(env: &Env) -> (Address, ForgetfulClient<'_>) {
    let id = env.register(Forgetful, ());
    let client = ForgetfulClient::new(env, &id);
    (id, client)
}

fn setup_careful(env: &Env) -> (Address, CarefulClient<'_>) {
    let id = env.register(Careful, ());
    let client = CarefulClient::new(env, &id);
    (id, client)
}

#[test]
fn lists_instance_code_and_storage_entries() {
    let env = Env::default();
    let (id, client) = setup_forgetful(&env);
    client.deposit(&Address::generate(&env), &10);

    let kinds: Vec<EntryKind> = Phoenix::new(&env)
        .watch(&id)
        .entries()
        .iter()
        .map(|e| e.kind)
        .collect();
    assert!(kinds.contains(&EntryKind::Instance));
    assert!(kinds.contains(&EntryKind::Code));
    assert!(kinds.contains(&EntryKind::Persistent));
    let balance = Phoenix::new(&env)
        .watch(&id)
        .entries()
        .into_iter()
        .find(|e| e.kind == EntryKind::Persistent)
        .unwrap();
    assert!(balance.key.starts_with("Balance(C"), "{}", balance.key);
}

#[test]
fn forgetful_contract_fails_the_live_for_check() {
    let env = Env::default();
    let (id, client) = setup_forgetful(&env);
    client.deposit(&Address::generate(&env), &10);

    let report = Phoenix::new(&env)
        .watch(&id)
        .check_live_for(days(7))
        .unwrap_err();
    assert_eq!(report.entries.len(), 3, "{report}");
    let text = report.to_string();
    assert!(
        text.contains("will not stay live for 120960 ledgers"),
        "{text}"
    );
    assert!(text.contains("instance().extend_ttl"), "{text}");
    assert!(text.contains("persistent().extend_ttl"), "{text}");
}

#[test]
#[should_panic(expected = "phoenix-ttl: 3 entries will not stay live")]
fn assert_live_for_panics_with_the_report() {
    let env = Env::default();
    let (id, client) = setup_forgetful(&env);
    client.deposit(&Address::generate(&env), &10);
    Phoenix::new(&env).watch(&id).assert_live_for(days(1));
}

#[test]
fn careful_contract_passes_and_ignores_temporary_by_default() {
    let env = Env::default();
    let (id, client) = setup_careful(&env);
    client.deposit(&Address::generate(&env), &10);

    let phoenix = Phoenix::new(&env).watch(&id);
    phoenix.assert_live_for(days(30));
    assert!(phoenix
        .entries()
        .iter()
        .all(|e| e.kind != EntryKind::Temporary));

    let with_temp = phoenix.clone().include_temporary(true);
    let err = with_temp.check_live_for(days(1)).unwrap_err();
    assert_eq!(err.entries.len(), 1);
    assert_eq!(err.entries[0].kind, EntryKind::Temporary);
    assert!(err.to_string().contains("deleted at ledger"));
}

#[test]
fn simulation_finds_the_first_step_that_loses_state() {
    let env = Env::default();
    let (id, client) = setup_forgetful(&env);
    let user = Address::generate(&env);
    client.deposit(&user, &10);

    // Default test TTLs are 4096 ledgers (≈ 5.7 hours), so daily use fails at step 1.
    let err = Phoenix::new(&env)
        .watch(&id)
        .simulate(days(1), 30, |_, _| client.deposit(&user, &1))
        .unwrap_err();
    assert_eq!(err.step, 1);
    assert_eq!(err.from, 0);
    assert_eq!(err.to, days(1));
    assert!(
        err.to_string()
            .contains("state would be lost before step 1"),
        "{err}"
    );
}

#[test]
fn careful_contract_survives_a_year_of_monthly_use() {
    let env = Env::default();
    let (id, client) = setup_careful(&env);
    let user = Address::generate(&env);
    client.deposit(&user, &10);

    let survival = Phoenix::new(&env)
        .watch(&id)
        .assert_survives(days(30), 12, |_, _| client.deposit(&user, &1));
    assert_eq!(survival.steps, 12);
    assert_eq!(survival.final_ledger, days(360));
    // The ledger timestamp advances with the sequence (5 s per ledger).
    assert_eq!(env.ledger().timestamp(), u64::from(days(360)) * 5);
    let (margin, _) = survival.closest_call.unwrap();
    assert!(margin >= days(30), "closest call was {margin}");
}

#[test]
fn careful_contract_loses_an_idle_users_balance() {
    // A user who deposits once and never comes back: their balance is
    // extended to 60 days and then archived, while the instance stays live
    // because other users keep calling.
    let env = Env::default();
    let (id, client) = setup_careful(&env);
    let idle = Address::generate(&env);
    let active = Address::generate(&env);
    client.deposit(&idle, &10);

    let err = Phoenix::new(&env)
        .watch(&id)
        .simulate(days(30), 12, |_, _| client.deposit(&active, &1))
        .unwrap_err();
    assert_eq!(err.step, 3);
    assert_eq!(err.entries.len(), 1);
    assert!(err.entries[0].key.starts_with("Balance("));
}

#[test]
fn watching_filters_other_contracts() {
    let env = Env::default();
    let (forgetful, f) = setup_forgetful(&env);
    let (careful, c) = setup_careful(&env);
    let u = Address::generate(&env);
    f.deposit(&u, &1);
    c.deposit(&u, &1);

    assert!(Phoenix::new(&env)
        .watch(&careful)
        .check_live_for(days(7))
        .is_ok());
    assert!(Phoenix::new(&env)
        .watch(&forgetful)
        .check_live_for(days(7))
        .is_err());
    // Watching nothing means watching everything.
    assert!(Phoenix::new(&env).check_live_for(days(7)).is_err());
}

#[test]
fn report_lists_entries_soonest_first() {
    let env = Env::default();
    let (id, client) = setup_careful(&env);
    client.deposit(&Address::generate(&env), &10);
    env.ledger().set_sequence_number(100);
    let report = Phoenix::new(&env).watch(&id).report();
    assert_eq!(report.current, 100);
    let lives: Vec<u32> = report.entries.iter().map(|e| e.live_until).collect();
    let mut sorted = lives.clone();
    sorted.sort();
    assert_eq!(lives, sorted);
    assert!(report.to_string().starts_with("phoenix-ttl: "));
}
