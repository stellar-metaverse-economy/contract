#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, String, Symbol};

#[contracttype]
#[derive(Clone)]
pub struct Region {
    pub id: u64,
    pub name: String,
    pub region_type: u32,
    pub total_shares: u128,
    pub productivity_score: u32,
    pub tax_rate: u32,
    pub governor: Address,
}

#[contracttype]
pub enum DataKey {
    Region(u64),
    RegionCount,
    Admin,
}

const REGION_CREATED: Symbol = symbol_short!("REG_NEW");

#[contract]
pub struct RegionFactory;

#[contractimpl]
impl RegionFactory {
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::RegionCount, &0u64);
    }

    pub fn create_region(
        env: Env,
        name: String,
        region_type: u32,
        total_shares: u128,
        tax_rate: u32,
        governor: Address,
    ) -> u64 {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let count: u64 = env.storage().instance().get(&DataKey::RegionCount).unwrap_or(0);
        let id = count + 1;

        let region = Region {
            id,
            name,
            region_type,
            total_shares,
            productivity_score: 100,
            tax_rate,
            governor,
        };

        env.storage().persistent().set(&DataKey::Region(id), &region);
        env.storage().instance().set(&DataKey::RegionCount, &id);
        env.events().publish((REGION_CREATED, id), region.total_shares);
        id
    }

    pub fn get_region(env: Env, id: u64) -> Region {
        env.storage().persistent().get(&DataKey::Region(id)).expect("region not found")
    }

    pub fn set_tax_rate(env: Env, id: u64, tax_rate: u32) {
        let mut region: Region = env.storage().persistent().get(&DataKey::Region(id)).expect("region not found");
        region.governor.require_auth();
        assert!(tax_rate <= 100, "tax rate must be <= 100");
        region.tax_rate = tax_rate;
        env.storage().persistent().set(&DataKey::Region(id), &region);
    }

    pub fn upgrade_region(env: Env, id: u64, productivity_delta: u32) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        let mut region: Region = env.storage().persistent().get(&DataKey::Region(id)).expect("region not found");
        region.productivity_score = region.productivity_score.saturating_add(productivity_delta);
        env.storage().persistent().set(&DataKey::Region(id), &region);
    }

    pub fn region_count(env: Env) -> u64 {
        env.storage().instance().get(&DataKey::RegionCount).unwrap_or(0)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::Env;

    #[test]
    fn test_create_and_get_region() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(RegionFactory, ());
        let client = RegionFactoryClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let governor = Address::generate(&env);
        client.initialize(&admin);

        let id = client.create_region(
            &String::from_str(&env, "Neo Lagos Port"),
            &1u32,
            &100_000u128,
            &10u32,
            &governor,
        );
        assert_eq!(id, 1);

        let region = client.get_region(&1);
        assert_eq!(region.total_shares, 100_000u128);
        assert_eq!(region.productivity_score, 100);
    }

    #[test]
    fn test_set_tax_rate() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(RegionFactory, ());
        let client = RegionFactoryClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let governor = Address::generate(&env);
        client.initialize(&admin);
        client.create_region(&String::from_str(&env, "Red Desert"), &2u32, &50_000u128, &5u32, &governor);
        client.set_tax_rate(&1, &20u32);
        assert_eq!(client.get_region(&1).tax_rate, 20);
    }
}
