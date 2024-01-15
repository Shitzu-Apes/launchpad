use crate::{
    errors,
    utils::{AFTER_FT_TRANSFER_GAS, FT_TRANSFER_GAS, ONE_YOCTO},
    Contract, ContractExt,
};
use near_contract_standards::fungible_token::core::ext_ft_core;
use near_sdk::{
    env, ext_contract, is_promise_success,
    json_types::U128,
    log, near_bindgen,
    serde::{Deserialize, Serialize},
    AccountId, NearToken, Promise,
};

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub enum FtOnTransferArgs {
    AccountDeposit,
}

#[ext_contract(ext_permission_contract)]
trait ExtPermissionContract {
    fn is_approved(&mut self, account_id: AccountId, sale_id: u64);
}

impl Contract {
    pub fn internal_ft_transfer(
        &mut self,
        account_id: &AccountId,
        token_account_id: &AccountId,
        amount: u128,
    ) -> Promise {
        ext_ft_core::ext(token_account_id.clone())
            .with_attached_deposit(ONE_YOCTO)
            .with_static_gas(FT_TRANSFER_GAS)
            .ft_transfer(account_id.clone(), amount.into(), None)
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(AFTER_FT_TRANSFER_GAS)
                    .after_ft_transfer(account_id.clone(), token_account_id.clone(), amount.into()),
            )
    }
}

#[near_bindgen]
impl Contract {
    #[private]
    pub fn after_ft_transfer(
        &mut self,
        account_id: AccountId,
        token_account_id: AccountId,
        amount: U128,
    ) -> bool {
        let promise_success = is_promise_success();
        if !is_promise_success() {
            log!(
                "{} by {} token {} amount {}",
                errors::TOKEN_WITHDRAW_FAILED,
                account_id,
                token_account_id,
                amount.0
            );
            let mut account = self.internal_unwrap_account(&account_id);
            account.internal_token_deposit(&token_account_id, amount.0);
        }
        promise_success
    }

    #[private]
    pub fn after_near_deposit(&mut self, amount: U128) -> bool {
        let promise_success = is_promise_success();
        if promise_success {
            log!(
                "Successfully wrapped {} NEAR tokens into Treasury",
                amount.0,
            );
            let w_near_token_id = self.treasury.w_near_token_id.clone();
            self.treasury.internal_deposit(&w_near_token_id, amount.0);
        }
        promise_success
    }

    #[private]
    pub fn after_is_approved(
        &mut self,
        #[callback_unwrap] is_approved: bool,
        sale_id: u64,
        account_id: AccountId,
        in_amount: U128,
        referral_id: Option<AccountId>,
        attached_deposit: U128,
    ) {
        assert!(is_approved, "{}", errors::NOT_APPROVED);
        let initial_storage_usage = env::storage_usage();

        assert!(self
            .internal_deposit_in_amount(
                sale_id,
                &account_id,
                in_amount.0,
                referral_id.as_ref(),
                true,
            )
            .is_none());

        let attached_deposit = attached_deposit.0;
        let required_cost = env::storage_byte_cost().as_yoctonear()
            * (env::storage_usage() - initial_storage_usage) as u128;
        assert!(
            required_cost <= attached_deposit,
            "{} {}",
            errors::NOT_ENOUGH_ATTACHED_BALANCE,
            required_cost,
        );

        let refund = attached_deposit - required_cost;
        if refund > 1 {
            Promise::new(account_id).transfer(NearToken::from_yoctonear(refund));
        }
        self.treasury.locked_attached_deposits -= attached_deposit;
    }

    #[private]
    pub fn maybe_refund_deposit(&mut self, account_id: AccountId, attached_deposit: U128) -> bool {
        let promise_success = is_promise_success();
        if !promise_success {
            self.treasury.locked_attached_deposits -= attached_deposit.0;
            Promise::new(account_id).transfer(NearToken::from_yoctonear(attached_deposit.0));
        }
        promise_success
    }
}
