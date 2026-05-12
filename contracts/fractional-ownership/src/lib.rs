#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, Symbol};

#[contracttype]
pub enum DataKey {
    Balance(u64, Address), // (region_id, owner)
    TotalShares(u64),
    Admin,
}

const TRANSFER: Symbol = symbol_short!("TRANSFER");

#[contract]
pub struct FractionalOwnership;

#[contractimpl]
impl FractionalOwnership {
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    /// Mint initial shares for a region to an owner (admin only)
    pub fn mint(env: Env, region_id: u64, to: Address, amount: u128) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let current: u128 = env.storage().persistent()
            .get(&DataKey::Balance(region_id, to.clone()))
            .unwrap_or(0);
        env.storage().persistent().set(&DataKey::Balance(region_id, to), &(current + amount));

        let total: u128 = env.storage().persistent()
            .get(&DataKey::TotalShares(region_id))
            .unwrap_or(0);
        env.storage().persistent().set(&DataKey::TotalShares(region_id), &(total + amount));
    }

    pub fn transfer(env: Env, region_id: u64, from: Address, to: Address, amount: u128) {
        from.require_auth();
        let from_bal: u128 = env.storage().persistent()
            .get(&DataKey::Balance(region_id, from.clone()))
            .unwrap_or(0);
        assert!(from_bal >= amount, "insufficient balance");

        env.storage().persistent().set(&DataKey::Balance(region_id, from.clone()), &(from_bal - amount));

        let to_bal: u128 = env.storage().persistent()
            .get(&DataKey::Balance(region_id, to.clone()))
            .unwrap_or(0);
        env.storage().persistent().set(&DataKey::Balance(region_id, to.clone()), &(to_bal + amount));

        env.events().publish((TRANSFER, region_id), (from, to, amount));
    }

    pub fn balance(env: Env, region_id: u64, owner: Address) -> u128 {
        env.storage().persistent()
            .get(&DataKey::Balance(region_id, owner))
            .unwrap_or(0)
    }

    pub fn total_shares(env: Env, region_id: u64) -> u128 {
        env.storage().persistent()
            .get(&DataKey::TotalShares(region_id))
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::Env;

    #[test]
    fn test_mint_and_transfer() {
        let env = Env::default();
        env.mock_all_auths();
        let id = env.register(FractionalOwnership, ());
        let client = FractionalOwnershipClient::new(&env, &id);

        let admin = Address::generate(&env);
        let alice = Address::generate(&env);
        let bob = Address::generate(&env);

        client.initialize(&admin);
        client.mint(&1u64, &alice, &10_000u128);
        assert_eq!(client.balance(&1u64, &alice), 10_000u128);
        assert_eq!(client.total_shares(&1u64), 10_000u128);

        client.transfer(&1u64, &alice, &bob, &3_000u128);
        assert_eq!(client.balance(&1u64, &alice), 7_000u128);
        assert_eq!(client.balance(&1u64, &bob), 3_000u128);
    }
}
