//! Logic to upgrade Sputnik contracts.

use near_sdk::serde_json::json;
use near_sdk::Gas;

use crate::*;

const FACTORY_KEY: &[u8; 7] = b"FACTORY";
const ERR_MUST_BE_SELF_OR_FACTORY: &str = "ERR_MUST_BE_SELF_OR_FACTORY";
const UPDATE_GAS_LEFTOVER: Gas = Gas(10_000_000_000_000);
const FACTORY_UPDATE_GAS_LEFTOVER: Gas = Gas(15_000_000_000_000);
const NO_DEPOSIT: Balance = 0;

pub const GAS_FOR_UPGRADE_SELF_DEPLOY: Gas = Gas(15_000_000_000_000);
pub const GAS_FOR_UPGRADE_REMOTE_DEPLOY: Gas = Gas(15_000_000_000_000);

/// Info about factory that deployed this contract and if auto-update is allowed.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[cfg_attr(not(target_arch = "wasm32"), derive(Clone, Debug))]
#[serde(crate = "near_sdk::serde")]
pub struct FactoryInfo {
    pub factory_id: AccountId,
    pub auto_update: bool,
}

pub fn get_default_factory_id() -> AccountId {
    // ex: mydao.sputnik-dao.near
    let dao_id = env::current_account_id().to_string();
    let idx = dao_id.find('.').expect("INTERNAL_FAIL");
    // ex: sputnik-dao.near
    let factory_id = &dao_id[idx + 1..];

    AccountId::new_unchecked(String::from(factory_id))
}

/// Fetches factory info from the storage.
/// By design not using contract STATE to allow for upgrade of stuck contracts from factory.
pub(crate) fn internal_get_factory_info() -> FactoryInfo {
    env::storage_read(FACTORY_KEY)
        .map(|value| FactoryInfo::try_from_slice(&value).expect("INTERNAL_FAIL"))
        .unwrap_or_else(|| FactoryInfo {
            factory_id: get_default_factory_id(),
            auto_update: true,
        })
}

pub(crate) fn internal_set_factory_info(factory_info: &FactoryInfo) {
    env::storage_write(
        FACTORY_KEY,
        &factory_info.try_to_vec().expect("INTERNAL_FAIL"),
    );
}

pub(crate) fn upgrade_using_factory(code_hash: Base58CryptoHash) {
    let account_id = get_default_factory_id();
    // Create a promise toward the factory.
    let promise_id = env::promise_batch_create(&account_id);
    // Call `update` method from the factory which calls `update` method on this account.
    env::promise_batch_action_function_call(
        promise_id,
        "update",
        &json!({ "account_id": env::current_account_id(), "code_hash": code_hash })
            .to_string()
            .into_bytes(),
        NO_DEPOSIT,
        env::prepaid_gas() - env::used_gas() - FACTORY_UPDATE_GAS_LEFTOVER,
    );
    env::promise_return(promise_id);
}
