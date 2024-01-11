use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupSet;
use near_sdk::{env, near_bindgen, AccountId, BorshStorageKey, PanicOnDefault};

#[derive(BorshStorageKey, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
pub(crate) enum StorageKey {
    Accounts,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
#[borsh(crate = "near_sdk::borsh")]
pub struct Contract {
    pub approved_accounts: LookupSet<AccountId>,

    pub owner_id: AccountId,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(owner_id: AccountId) -> Self {
        Self {
            approved_accounts: LookupSet::new(StorageKey::Accounts),
            owner_id,
        }
    }

    pub fn is_permissions_contract(&self) -> bool {
        true
    }

    #[allow(unused_variables)]
    pub fn is_approved(&self, account_id: AccountId, sale_id: u64) -> bool {
        self.approved_accounts.contains(&account_id)
    }

    pub fn approve(&mut self, account_id: AccountId) {
        self.assert_called_by_owner();
        self.approved_accounts.insert(&account_id);
    }

    pub fn reject(&mut self, account_id: AccountId) {
        self.assert_called_by_owner();
        self.approved_accounts.remove(&account_id);
    }
}

impl Contract {
    fn assert_called_by_owner(&self) {
        assert_eq!(&self.owner_id, &env::predecessor_account_id());
    }
}
