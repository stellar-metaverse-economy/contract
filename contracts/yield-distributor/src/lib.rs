#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, Symbol};

#[contracttype]
pub enum DataKey {
    Admin,
    RevenuePool(u64),
    Claimed(u64, Address),
    TotalShares(u64),
    Balance(u64, Address),
}

const YIELD_DIST: Symbol = symbol_short!("YIELD");

#[contract]
pub struct YieldDistributor;

#[contractimpl]
impl YieldDistributor {
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    /// Deposit revenue into a region's pool (admin/game engine)
    pub fn deposit_revenue(env: Env, region_id: u64, amount: i128) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        assert!(amount > 0, "amount must be positive");

        let pool: i128 = env.storage().persistent()
            .get(&DataKey::RevenuePool(region_id))
            .unwrap_or(0);
        env.storage().persistent().set(&DataKey::RevenuePool(region_id), &(pool + amount));
        env.events().publish((YIELD_DIST, region_id), amount);
    }

    /// Set snapshot of shares for a region (called by admin after ownership snapshot)
    pub fn set_shares(env: Env, region_id: u64, owner: Address, shares: u128, total: u128) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        env.storage().persistent().set(&DataKey::Balance(region_id, owner), &shares);
        env.storage().persistent().set(&DataKey::TotalShares(region_id), &total);
    }

    /// Claim proportional yield for a player
    pub fn claim(env: Env, region_id: u64, claimant: Address) -> i128 {
        claimant.require_auth();

        let pool: i128 = env.storage().persistent()
            .get(&DataKey::RevenuePool(region_id))
            .unwrap_or(0);
        let total: u128 = env.storage().persistent()
            .get(&DataKey::TotalShares(region_id))
            .unwrap_or(0);
        let shares: u128 = env.storage().persistent()
            .get(&DataKey::Balance(region_id, claimant.clone()))
            .unwrap_or(0);

        assert!(total > 0, "no shares");
        assert!(shares > 0, "no shares owned");

        let already_claimed: i128 = env.storage().persistent()
            .get(&DataKey::Claimed(region_id, claimant.clone()))
            .unwrap_or(0);

        // payout = (shares / total) * pool - already_claimed
        let entitled = (pool as u128) * shares / total;
        let payout = (entitled as i128) - already_claimed;
        assert!(payout > 0, "nothing to claim");

        env.storage().persistent().set(&DataKey::Claimed(region_id, claimant), &(already_claimed + payout));
        payout
    }

    pub fn claimable(env: Env, region_id: u64, claimant: Address) -> i128 {
        let pool: i128 = env.storage().persistent()
            .get(&DataKey::RevenuePool(region_id))
            .unwrap_or(0);
        let total: u128 = env.storage().persistent()
            .get(&DataKey::TotalShares(region_id))
            .unwrap_or(0);
        let shares: u128 = env.storage().persistent()
            .get(&DataKey::Balance(region_id, claimant.clone()))
            .unwrap_or(0);
        if total == 0 || shares == 0 { return 0; }
        let already_claimed: i128 = env.storage().persistent()
            .get(&DataKey::Claimed(region_id, claimant))
            .unwrap_or(0);
        let entitled = (pool as u128) * shares / total;
        (entitled as i128) - already_claimed
    }

    pub fn revenue_pool(env: Env, region_id: u64) -> i128 {
        env.storage().persistent().get(&DataKey::RevenuePool(region_id)).unwrap_or(0)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::Env;

    #[test]
    fn test_deposit_and_claim() {
        let env = Env::default();
        env.mock_all_auths();
        let id = env.register(YieldDistributor, ());
        let client = YieldDistributorClient::new(&env, &id);

        let admin = Address::generate(&env);
        let alice = Address::generate(&env);

        client.initialize(&admin);
        // Alice owns 5000 of 100000 shares = 5%
        client.set_shares(&1u64, &alice, &5_000u128, &100_000u128);
        client.deposit_revenue(&1u64, &10_000i128);

        let claimable = client.claimable(&1u64, &alice);
        assert_eq!(claimable, 500); // 5% of 10000

        let payout = client.claim(&1u64, &alice);
        assert_eq!(payout, 500);

        // Nothing left to claim
        assert_eq!(client.claimable(&1u64, &alice), 0);
    }
}
