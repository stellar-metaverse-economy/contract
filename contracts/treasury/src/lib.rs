#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, Symbol};

#[contracttype]
pub enum DataKey {
    Admin,
    Balance(u64),   // region_id -> balance
    TotalBalance,
}

const DEPOSIT: Symbol = symbol_short!("DEPOSIT");
const WITHDRAW: Symbol = symbol_short!("WITHDRAW");

#[contract]
pub struct Treasury;

#[contractimpl]
impl Treasury {
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::TotalBalance, &0i128);
    }

    pub fn deposit(env: Env, region_id: u64, amount: i128) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        assert!(amount > 0, "amount must be positive");

        let bal: i128 = env.storage().persistent().get(&DataKey::Balance(region_id)).unwrap_or(0);
        env.storage().persistent().set(&DataKey::Balance(region_id), &(bal + amount));

        let total: i128 = env.storage().instance().get(&DataKey::TotalBalance).unwrap_or(0);
        env.storage().instance().set(&DataKey::TotalBalance, &(total + amount));

        env.events().publish((DEPOSIT, region_id), amount);
    }

    pub fn withdraw(env: Env, region_id: u64, amount: i128, recipient: Address) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        assert!(amount > 0, "amount must be positive");

        let bal: i128 = env.storage().persistent().get(&DataKey::Balance(region_id)).unwrap_or(0);
        assert!(bal >= amount, "insufficient treasury balance");

        env.storage().persistent().set(&DataKey::Balance(region_id), &(bal - amount));

        let total: i128 = env.storage().instance().get(&DataKey::TotalBalance).unwrap_or(0);
        env.storage().instance().set(&DataKey::TotalBalance, &(total - amount));

        env.events().publish((WITHDRAW, region_id), (recipient, amount));
    }

    pub fn balance(env: Env, region_id: u64) -> i128 {
        env.storage().persistent().get(&DataKey::Balance(region_id)).unwrap_or(0)
    }

    pub fn total_balance(env: Env) -> i128 {
        env.storage().instance().get(&DataKey::TotalBalance).unwrap_or(0)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::Env;

    #[test]
    fn test_deposit_and_withdraw() {
        let env = Env::default();
        env.mock_all_auths();
        let id = env.register(Treasury, ());
        let client = TreasuryClient::new(&env, &id);

        let admin = Address::generate(&env);
        let recipient = Address::generate(&env);

        client.initialize(&admin);
        client.deposit(&1u64, &5_000i128);
        client.deposit(&1u64, &3_000i128);
        assert_eq!(client.balance(&1u64), 8_000i128);
        assert_eq!(client.total_balance(), 8_000i128);

        client.withdraw(&1u64, &2_000i128, &recipient);
        assert_eq!(client.balance(&1u64), 6_000i128);
        assert_eq!(client.total_balance(), 6_000i128);
    }
}
