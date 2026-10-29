//! A minimal token vault used to demonstrate phoenix-ttl.
//!
//! TTL policy (the thing the Phoenix tests pin down):
//! - The contract instance and code are extended on every call, so the
//!   vault stays live as long as anyone uses it at least every 30 days.
//! - A user's share balance is extended whenever *that user* deposits or
//!   withdraws. A user who goes quiet for more than ~60 days will have
//!   their balance archived and must restore it (paying the fee) before
//!   withdrawing. This is a deliberate trade-off: the vault does not pay
//!   rent for inactive users.
#![no_std]

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, token, Address, Env,
};

/// Ledgers per day at ~5 seconds per ledger.
const DAY: u32 = 17_280;
/// Extend when fewer than 30 days remain…
const BUMP_THRESHOLD: u32 = 30 * DAY;
/// …to 60 days.
const BUMP_TO: u32 = 60 * DAY;

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Token,
    TotalShares,
    Shares(Address),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum VaultError {
    InvalidAmount = 1,
    InsufficientShares = 2,
}

#[contractevent]
pub struct Deposited {
    #[topic]
    pub user: Address,
    pub amount: i128,
}

#[contract]
pub struct Vault;

#[contractimpl]
impl Vault {
    pub fn __constructor(env: Env, token: Address) {
        env.storage().instance().set(&DataKey::Token, &token);
        env.storage().instance().set(&DataKey::TotalShares, &0i128);
        bump_instance(&env);
    }

    pub fn deposit(env: Env, user: Address, amount: i128) -> Result<i128, VaultError> {
        user.require_auth();
        if amount <= 0 {
            return Err(VaultError::InvalidAmount);
        }
        bump_instance(&env);
        token_client(&env).transfer(&user, env.current_contract_address(), &amount);
        let shares = shares_of(&env, &user) + amount;
        set_shares(&env, &user, shares);
        add_total(&env, amount);
        Deposited { user, amount }.publish(&env);
        Ok(shares)
    }

    pub fn withdraw(env: Env, user: Address, amount: i128) -> Result<i128, VaultError> {
        user.require_auth();
        if amount <= 0 {
            return Err(VaultError::InvalidAmount);
        }
        bump_instance(&env);
        let shares = shares_of(&env, &user);
        if shares < amount {
            return Err(VaultError::InsufficientShares);
        }
        set_shares(&env, &user, shares - amount);
        add_total(&env, -amount);
        token_client(&env).transfer(&env.current_contract_address(), &user, &amount);
        Ok(shares - amount)
    }

    pub fn shares(env: Env, user: Address) -> i128 {
        shares_of(&env, &user)
    }
}

/// Extends the contract instance *and* its Wasm code.
fn bump_instance(env: &Env) {
    env.storage().instance().extend_ttl(BUMP_THRESHOLD, BUMP_TO);
}

fn token_client(env: &Env) -> token::Client<'_> {
    let token: Address = env.storage().instance().get(&DataKey::Token).unwrap();
    token::Client::new(env, &token)
}

fn shares_of(env: &Env, user: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&DataKey::Shares(user.clone()))
        .unwrap_or(0)
}

fn set_shares(env: &Env, user: &Address, shares: i128) {
    let key = DataKey::Shares(user.clone());
    env.storage().persistent().set(&key, &shares);
    env.storage()
        .persistent()
        .extend_ttl(&key, BUMP_THRESHOLD, BUMP_TO);
}

fn add_total(env: &Env, delta: i128) {
    let total: i128 = env
        .storage()
        .instance()
        .get(&DataKey::TotalShares)
        .unwrap_or(0);
    env.storage()
        .instance()
        .set(&DataKey::TotalShares, &(total + delta));
}

#[cfg(test)]
mod test;
