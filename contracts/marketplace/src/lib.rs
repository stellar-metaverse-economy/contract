#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, Symbol};

#[contracttype]
#[derive(Clone)]
pub struct Listing {
    pub seller: Address,
    pub region_id: u64,
    pub shares: u128,
    pub price_per_share: i128, // in stroops
    pub active: bool,
}

#[contracttype]
pub enum DataKey {
    Admin,
    Listing(u64),   // listing_id -> Listing
    ListingCount,
}

const LISTED: Symbol = symbol_short!("LISTED");
const SOLD: Symbol = symbol_short!("SOLD");

#[contract]
pub struct Marketplace;

#[contractimpl]
impl Marketplace {
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::ListingCount, &0u64);
    }

    pub fn list_shares(
        env: Env,
        seller: Address,
        region_id: u64,
        shares: u128,
        price_per_share: i128,
    ) -> u64 {
        seller.require_auth();
        assert!(shares > 0 && price_per_share > 0, "invalid listing");

        let count: u64 = env.storage().instance().get(&DataKey::ListingCount).unwrap_or(0);
        let listing_id = count + 1;

        let listing = Listing { seller, region_id, shares, price_per_share, active: true };
        env.storage().persistent().set(&DataKey::Listing(listing_id), &listing);
        env.storage().instance().set(&DataKey::ListingCount, &listing_id);
        env.events().publish((LISTED, listing_id), region_id);
        listing_id
    }

    /// Buy shares from a listing. Returns total cost.
    pub fn buy_shares(env: Env, buyer: Address, listing_id: u64, amount: u128) -> i128 {
        buyer.require_auth();
        let mut listing: Listing = env.storage().persistent()
            .get(&DataKey::Listing(listing_id))
            .expect("listing not found");
        assert!(listing.active, "listing not active");
        assert!(amount <= listing.shares, "not enough shares in listing");

        let cost = listing.price_per_share * amount as i128;
        listing.shares -= amount;
        if listing.shares == 0 {
            listing.active = false;
        }
        env.storage().persistent().set(&DataKey::Listing(listing_id), &listing);
        env.events().publish((SOLD, listing_id), (buyer, amount, cost));
        cost
    }

    pub fn cancel_listing(env: Env, listing_id: u64) {
        let mut listing: Listing = env.storage().persistent()
            .get(&DataKey::Listing(listing_id))
            .expect("listing not found");
        listing.seller.require_auth();
        listing.active = false;
        env.storage().persistent().set(&DataKey::Listing(listing_id), &listing);
    }

    pub fn get_listing(env: Env, listing_id: u64) -> Listing {
        env.storage().persistent().get(&DataKey::Listing(listing_id)).expect("not found")
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::Env;

    #[test]
    fn test_list_and_buy() {
        let env = Env::default();
        env.mock_all_auths();
        let id = env.register(Marketplace, ());
        let client = MarketplaceClient::new(&env, &id);

        let admin = Address::generate(&env);
        let seller = Address::generate(&env);
        let buyer = Address::generate(&env);

        client.initialize(&admin);
        let listing_id = client.list_shares(&seller, &1u64, &1_000u128, &100i128);
        assert_eq!(listing_id, 1);

        let cost = client.buy_shares(&buyer, &1u64, &500u128);
        assert_eq!(cost, 50_000i128);

        let listing = client.get_listing(&1u64);
        assert_eq!(listing.shares, 500u128);
        assert!(listing.active);
    }

    #[test]
    fn test_cancel_listing() {
        let env = Env::default();
        env.mock_all_auths();
        let id = env.register(Marketplace, ());
        let client = MarketplaceClient::new(&env, &id);

        let admin = Address::generate(&env);
        let seller = Address::generate(&env);
        client.initialize(&admin);
        client.list_shares(&seller, &2u64, &500u128, &50i128);
        client.cancel_listing(&1u64);
        assert!(!client.get_listing(&1u64).active);
    }
}
