use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{U128, U64};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{env, near_bindgen, AccountId, Promise, PromiseOrValue};

use crate::types::{convert_old_to_new_token, OldAccountId};
use crate::*;

/// Information recorded about claim of the bounty by given user.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct BountyClaim {
    /// Bounty id that was claimed.
    bounty_id: u64,
    /// Start time of the claim.
    start_time: U64,
    /// Deadline specified by claimer.
    deadline: U64,
    /// Completed?
    completed: bool,
}

/// Bounty information.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug))]
#[serde(crate = "near_sdk::serde")]
pub struct Bounty {
    /// Description of the bounty.
    pub description: String,
    /// Token the bounty will be paid out.
    /// Can be "" for $NEAR or a valid account id.
    pub token: OldAccountId,
    /// Amount to be paid out.
    pub amount: U128,
    /// How many times this bounty can be done.
    pub times: u32,
    /// Max deadline from claim that can be spend on this bounty.
    pub max_deadline: U64,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[cfg_attr(not(target_arch = "wasm32"), derive(Clone, Debug))]
#[serde(crate = "near_sdk::serde")]
pub enum VersionedBounty {
    Default(Bounty),
}

impl From<VersionedBounty> for Bounty {
    fn from(v: VersionedBounty) -> Self {
        match v {
            VersionedBounty::Default(b) => b,
        }
    }
}

#[cfg(test)]
mod tests {
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;
    use near_sdk_sim::to_yocto;

    use crate::proposals::{ProposalInput, ProposalKind};
    use crate::{Action, Config};

    use super::*;

    fn add_bounty(context: &mut VMContextBuilder, contract: &mut Contract, times: u32) -> u64 {
        testing_env!(context.attached_deposit(to_yocto("1")).build());
        let id = contract.add_proposal(ProposalInput {
            description: "test".to_string(),
            kind: ProposalKind::AddBounty {
                bounty: Bounty {
                    description: "test bounty".to_string(),
                    token: String::from(OLD_BASE_TOKEN),
                    amount: U128(to_yocto("10")),
                    times,
                    max_deadline: U64::from(1_000),
                },
            },
        });
        assert_eq!(contract.get_last_bounty_id(), id);
        contract.act_proposal(id, Action::VoteApprove, None);
        id
    }

    /// Adds a bounty, and tests it's full lifecycle.
    #[test]
    fn test_bounty_lifecycle() {
        let mut context = VMContextBuilder::new();
        testing_env!(context.predecessor_account_id(accounts(1)).build());
        let mut contract = Contract::new(
            Config::test_config(),
            VersionedPolicy::Default(vec![accounts(1).into()]),
        );
        add_bounty(&mut context, &mut contract, 2);

        assert_eq!(contract.get_last_bounty_id(), 1);
        assert_eq!(contract.get_bounty(0).bounty.times, 2);

        contract.bounty_claim(0, U64::from(500));
        assert_eq!(contract.get_bounty_claims(accounts(1)).len(), 1);
        assert_eq!(contract.get_bounty_number_of_claims(0), 1);

        contract.bounty_giveup(0);
        assert_eq!(contract.get_bounty_claims(accounts(1)).len(), 0);
        assert_eq!(contract.get_bounty_number_of_claims(0), 0);

        contract.bounty_claim(0, U64::from(500));
        assert_eq!(contract.get_bounty_claims(accounts(1)).len(), 1);
        assert_eq!(contract.get_bounty_number_of_claims(0), 1);

        contract.bounty_done(0, None, "Bounty is done".to_string());
        assert!(contract.get_bounty_claims(accounts(1))[0].completed);

        assert_eq!(contract.get_last_proposal_id(), 2);
        assert_eq!(
            contract.get_proposal(1).proposal.kind.to_policy_label(),
            "bounty_done"
        );

        contract.act_proposal(1, Action::VoteApprove, None);
        testing_env!(
            context.build(),
            near_sdk::VMConfig::test(),
            near_sdk::RuntimeFeesConfig::test(),
            Default::default(),
            vec![PromiseResult::Successful(vec![])],
        );
        contract.on_proposal_callback(1);

        assert_eq!(contract.get_bounty_claims(accounts(1)).len(), 0);
        assert_eq!(contract.get_bounty(0).bounty.times, 1);

        contract.bounty_claim(0, U64::from(500));
        contract.bounty_done(0, None, "Bounty is done 2".to_string());
        contract.act_proposal(2, Action::VoteApprove, None);
        testing_env!(
            context.build(),
            near_sdk::VMConfig::test(),
            near_sdk::RuntimeFeesConfig::test(),
            Default::default(),
            vec![PromiseResult::Successful(vec![])],
        );
        contract.on_proposal_callback(2);

        assert_eq!(contract.get_bounty(0).bounty.times, 0);
    }

    #[test]
    #[should_panic(expected = "ERR_BOUNTY_ALL_CLAIMED")]
    fn test_bounty_claim_not_allowed() {
        let mut context = VMContextBuilder::new();
        testing_env!(context.predecessor_account_id(accounts(1)).build());
        let mut contract = Contract::new(
            Config::test_config(),
            VersionedPolicy::Default(vec![accounts(1).into()]),
        );
        let id = add_bounty(&mut context, &mut contract, 1);
        contract.bounty_claim(id, U64::from(500));
        contract.bounty_done(id, None, "Bounty is done 2".to_string());
        contract.bounty_claim(id, U64::from(500));
    }
}
