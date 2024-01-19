use crate::{
    errors, Contract, ContractExt, StorageKey, AFTER_CLAIM_TREASURY_GAS, AFTER_NEAR_DEPOSIT_GAS,
    EXTRA_NEAR, NEAR_DEPOSIT_GAS, STORAGE_DEPOSIT, STORAGE_DEPOSIT_GAS,
};
use near_contract_standards::fungible_token::core::ext_ft_core;
use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    collections::UnorderedMap,
    env,
    json_types::U128,
    near_bindgen, AccountId, NearToken, Promise, PromiseOrValue, PromiseResult,
};

#[derive(BorshDeserialize, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
pub struct Treasury {
    pub balances: UnorderedMap<AccountId, u128>,

    pub listing_fee_near: u128,

    pub w_near_token_id: AccountId,

    // The amount of NEAR locked while the permissions are being verified.
    pub locked_attached_deposits: u128,
}

impl Treasury {
    pub fn new(listing_fee_near: u128, w_near_token_id: AccountId) -> Self {
        Self {
            balances: UnorderedMap::new(StorageKey::TreasuryBalances),
            listing_fee_near,
            w_near_token_id,
            locked_attached_deposits: 0,
        }
    }

    pub fn internal_deposit(&mut self, token_account_id: &AccountId, amount: u128) {
        let balance = self.balances.get(token_account_id).unwrap_or(0);
        let new_balance = balance.checked_add(amount).expect(errors::BALANCE_OVERFLOW);
        self.balances.insert(token_account_id, &new_balance);
    }

    pub fn internal_withdraw(&mut self, token_account_id: &AccountId, amount: u128) {
        let balance = self.balances.get(token_account_id).unwrap_or(0);
        let new_balance = balance
            .checked_sub(amount)
            .expect(errors::NOT_ENOUGH_BALANCE);
        self.balances.insert(token_account_id, &new_balance);
    }
}

#[near_bindgen]
impl Contract {
    pub fn claim_treasury(&mut self) -> PromiseOrValue<()> {
        let mut promise: Option<Promise> = None;
        let mut token_ids = Vec::with_capacity(self.treasury.balances.len() as usize);
        for (token_id, balance) in self.treasury.balances.iter() {
            if balance == 0 {
                continue;
            }
            token_ids.push(token_id.clone());
            if let Some(p) = promise {
                promise = Some(
                    p.and(
                        ext_ft_core::ext(token_id.clone())
                            .with_unused_gas_weight(1)
                            .with_attached_deposit(NearToken::from_yoctonear(1))
                            .ft_transfer(self.dao.clone(), balance.into(), None),
                    ),
                );
            } else {
                promise = Some(
                    ext_ft_core::ext(token_id.clone())
                        .with_unused_gas_weight(1)
                        .with_attached_deposit(NearToken::from_yoctonear(1))
                        .ft_transfer(self.dao.clone(), balance.into(), None),
                );
            }
        }
        if let Some(promise) = promise {
            PromiseOrValue::Promise(
                promise.then(
                    Self::ext(env::current_account_id())
                        .with_static_gas(AFTER_CLAIM_TREASURY_GAS)
                        .after_claim_treasury(token_ids),
                ),
            )
        } else {
            PromiseOrValue::Value(())
        }
    }

    #[private]
    pub fn after_claim_treasury(&mut self, token_ids: Vec<AccountId>) {
        for i in 0..env::promise_results_count() {
            if let PromiseResult::Successful(_) = env::promise_result(i) {
                self.treasury
                    .balances
                    .insert(token_ids.get(i as usize).unwrap(), &0);
            }
        }
    }

    pub fn get_treasury_balance(&self, token_account_id: AccountId) -> Option<U128> {
        self.treasury
            .balances
            .get(&token_account_id)
            .map(|a| a.into())
    }

    pub fn get_treasury_balances(
        &self,
        from_index: Option<u64>,
        limit: Option<u64>,
    ) -> Vec<(AccountId, U128)> {
        let keys = self.treasury.balances.keys_as_vector();
        let values = self.treasury.balances.values_as_vector();
        let from_index = from_index.unwrap_or(0);
        let limit = limit.unwrap_or(keys.len());
        (from_index..std::cmp::min(from_index + limit, keys.len()))
            .map(|index| (keys.get(index).unwrap(), values.get(index).unwrap().into()))
            .collect()
    }

    pub fn get_treasury_num_balances(&self) -> u64 {
        self.treasury.balances.len()
    }

    pub fn get_listing_fee(&self) -> U128 {
        self.treasury.listing_fee_near.into()
    }

    pub fn wrap_extra_near(&mut self) -> Promise {
        let unused_near_balance = env::account_balance().as_yoctonear()
            - env::storage_usage() as u128 * env::storage_byte_cost().as_yoctonear()
            - self.treasury.locked_attached_deposits;
        assert!(
            unused_near_balance
                > env::storage_byte_cost()
                    .checked_mul(EXTRA_NEAR)
                    .unwrap()
                    .as_yoctonear()
                    + NearToken::from_near(1).as_yoctonear(),
            "{}",
            errors::NOT_ENOUGH_BALANCE
        );
        let extra_near = unused_near_balance - EXTRA_NEAR * env::storage_byte_cost().as_yoctonear();
        Promise::new(self.treasury.w_near_token_id.clone())
            .function_call(
                "storage_deposit".to_string(),
                b"{}".to_vec(),
                env::storage_byte_cost()
                    .checked_mul(STORAGE_DEPOSIT)
                    .unwrap(),
                STORAGE_DEPOSIT_GAS,
            )
            .function_call(
                "near_deposit".to_string(),
                b"{}".to_vec(),
                NearToken::from_yoctonear(extra_near),
                NEAR_DEPOSIT_GAS,
            )
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(AFTER_NEAR_DEPOSIT_GAS)
                    .after_near_deposit(extra_near.into()),
            )
    }
}
