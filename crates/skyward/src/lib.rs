pub mod account;
pub(crate) mod errors;
mod internal;
pub mod sale;
pub mod sub;
pub mod treasury;
pub(crate) mod utils;

pub use crate::account::*;
pub use crate::internal::*;
pub use crate::sale::*;
pub use crate::sub::*;
pub use crate::treasury::*;
pub(crate) use crate::utils::*;

use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    collections::LookupMap,
    json_types::U128,
    near_bindgen, AccountId, BorshStorageKey, PanicOnDefault,
};

#[derive(BorshStorageKey, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
pub(crate) enum StorageKey {
    Accounts,
    AccountTokens { account_id: AccountId },
    AccountSubs { account_id: AccountId },
    AccountSales { account_id: AccountId },
    Sales,
    TreasuryBalances,
    VestingSchedule,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
#[borsh(crate = "near_sdk::borsh")]
pub struct Contract {
    pub accounts: LookupMap<AccountId, VAccount>,

    pub sales: LookupMap<u64, VSale>,

    pub num_sales: u64,

    pub treasury: Treasury,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(
        skyward_token_id: AccountId,
        skyward_vesting_schedule: Vec<VestingIntervalInput>,
        listing_fee_near: U128,
        w_near_token_id: AccountId,
    ) -> Self {
        Self {
            accounts: LookupMap::new(StorageKey::Accounts),
            sales: LookupMap::new(StorageKey::Sales),
            num_sales: 0,
            treasury: Treasury::new(
                skyward_token_id,
                skyward_vesting_schedule,
                listing_fee_near.0,
                w_near_token_id,
            ),
        }
    }
}
