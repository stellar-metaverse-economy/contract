#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, String, Symbol};

#[contracttype]
#[derive(Clone, PartialEq)]
pub enum ProposalStatus {
    Active,
    Passed,
    Rejected,
}

#[contracttype]
#[derive(Clone)]
pub struct Proposal {
    pub id: u64,
    pub region_id: u64,
    pub proposer: Address,
    pub description: String,
    pub votes_for: u128,
    pub votes_against: u128,
    pub status: ProposalStatus,
    pub end_ledger: u32,
}

#[contracttype]
pub enum DataKey {
    Admin,
    Proposal(u64),
    ProposalCount,
    Voted(u64, Address), // (proposal_id, voter)
    ShareBalance(u64, Address), // (region_id, voter) — snapshot
}

const PROPOSED: Symbol = symbol_short!("PROPOSED");
const VOTED: Symbol = symbol_short!("VOTED");

#[contract]
pub struct GovernanceDao;

#[contractimpl]
impl GovernanceDao {
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::ProposalCount, &0u64);
    }

    /// Set voting power snapshot for a voter in a region
    pub fn set_voting_power(env: Env, region_id: u64, voter: Address, shares: u128) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        env.storage().persistent().set(&DataKey::ShareBalance(region_id, voter), &shares);
    }

    pub fn create_proposal(
        env: Env,
        region_id: u64,
        proposer: Address,
        description: String,
        voting_period_ledgers: u32,
    ) -> u64 {
        proposer.require_auth();
        let count: u64 = env.storage().instance().get(&DataKey::ProposalCount).unwrap_or(0);
        let id = count + 1;
        let end_ledger = env.ledger().sequence() + voting_period_ledgers;

        let proposal = Proposal {
            id,
            region_id,
            proposer,
            description,
            votes_for: 0,
            votes_against: 0,
            status: ProposalStatus::Active,
            end_ledger,
        };
        env.storage().persistent().set(&DataKey::Proposal(id), &proposal);
        env.storage().instance().set(&DataKey::ProposalCount, &id);
        env.events().publish((PROPOSED, id), region_id);
        id
    }

    pub fn vote(env: Env, proposal_id: u64, voter: Address, support: bool) {
        voter.require_auth();
        assert!(
            !env.storage().persistent().has(&DataKey::Voted(proposal_id, voter.clone())),
            "already voted"
        );

        let mut proposal: Proposal = env.storage().persistent()
            .get(&DataKey::Proposal(proposal_id))
            .expect("proposal not found");
        assert!(proposal.status == ProposalStatus::Active, "not active");
        assert!(env.ledger().sequence() <= proposal.end_ledger, "voting ended");

        let power: u128 = env.storage().persistent()
            .get(&DataKey::ShareBalance(proposal.region_id, voter.clone()))
            .unwrap_or(0);
        assert!(power > 0, "no voting power");

        if support {
            proposal.votes_for += power;
        } else {
            proposal.votes_against += power;
        }

        env.storage().persistent().set(&DataKey::Voted(proposal_id, voter.clone()), &true);
        env.storage().persistent().set(&DataKey::Proposal(proposal_id), &proposal);
        env.events().publish((VOTED, proposal_id), (voter, support, power));
    }

    pub fn finalize(env: Env, proposal_id: u64) -> ProposalStatus {
        let mut proposal: Proposal = env.storage().persistent()
            .get(&DataKey::Proposal(proposal_id))
            .expect("proposal not found");
        assert!(proposal.status == ProposalStatus::Active, "already finalized");
        assert!(env.ledger().sequence() > proposal.end_ledger, "voting still active");

        proposal.status = if proposal.votes_for > proposal.votes_against {
            ProposalStatus::Passed
        } else {
            ProposalStatus::Rejected
        };
        env.storage().persistent().set(&DataKey::Proposal(proposal_id), &proposal);
        proposal.status
    }

    pub fn get_proposal(env: Env, proposal_id: u64) -> Proposal {
        env.storage().persistent().get(&DataKey::Proposal(proposal_id)).expect("not found")
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::Env;

    #[test]
    fn test_proposal_and_vote() {
        let env = Env::default();
        env.mock_all_auths();
        let id = env.register(GovernanceDao, ());
        let client = GovernanceDaoClient::new(&env, &id);

        let admin = Address::generate(&env);
        let alice = Address::generate(&env);
        let bob = Address::generate(&env);

        client.initialize(&admin);
        client.set_voting_power(&1u64, &alice, &6_000u128);
        client.set_voting_power(&1u64, &bob, &4_000u128);

        let pid = client.create_proposal(
            &1u64,
            &alice,
            &String::from_str(&env, "Raise tax to 15%"),
            &100u32,
        );
        assert_eq!(pid, 1);

        client.vote(&1u64, &alice, &true);
        client.vote(&1u64, &bob, &false);

        let proposal = client.get_proposal(&1u64);
        assert_eq!(proposal.votes_for, 6_000u128);
        assert_eq!(proposal.votes_against, 4_000u128);
    }
}
