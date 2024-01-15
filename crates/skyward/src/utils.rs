use crate::errors;
use near_sdk::{env, AccountId, Gas, NearToken, Promise, StorageUsage};

pub(crate) const ONE_YOCTO: NearToken = NearToken::from_yoctonear(1);

pub(crate) const STORAGE_DEPOSIT: u128 = 125;
pub(crate) const EXTRA_NEAR_FOR_STORAGE: u128 = 1000;
pub(crate) const EXTRA_NEAR: u128 = EXTRA_NEAR_FOR_STORAGE + STORAGE_DEPOSIT;

const BASE_GAS: Gas = Gas::from_tgas(5);
pub(crate) const FT_TRANSFER_GAS: Gas = BASE_GAS;
pub(crate) const AFTER_FT_TRANSFER_GAS: Gas = BASE_GAS;
pub(crate) const AFTER_NEAR_DEPOSIT_GAS: Gas = BASE_GAS;

pub(crate) const STORAGE_DEPOSIT_GAS: Gas = Gas::from_gas(BASE_GAS.as_gas() * 2);
pub(crate) const NEAR_DEPOSIT_GAS: Gas = BASE_GAS;

pub(crate) const PERMISSION_CONTRACT_GAS: Gas = Gas::from_gas(BASE_GAS.as_gas() * 10);
pub(crate) const AFTER_IS_APPROVED_GAS: Gas = Gas::from_gas(BASE_GAS.as_gas() * 4);
pub(crate) const MAYBE_REFUND_DEPOSIT_GAS: Gas = Gas::from_gas(BASE_GAS.as_gas() * 2);

pub type BasicPoints = u16;

pub(crate) fn refund_extra_storage_deposit(storage_used: StorageUsage, used_balance: u128) {
    let required_cost = env::storage_byte_cost().as_yoctonear() * storage_used as u128;
    let attached_deposit = env::attached_deposit()
        .checked_sub(NearToken::from_yoctonear(used_balance))
        .expect(errors::NOT_ENOUGH_ATTACHED_BALANCE)
        .as_yoctonear();

    assert!(
        required_cost <= attached_deposit,
        "{} {}",
        errors::NOT_ENOUGH_ATTACHED_BALANCE,
        required_cost,
    );

    let refund = attached_deposit - required_cost;
    if refund > 1 {
        Promise::new(env::predecessor_account_id()).transfer(NearToken::from_yoctonear(refund));
    }
}

pub(crate) fn refund_released_storage(account_id: &AccountId, storage_released: StorageUsage) {
    if storage_released > 0 {
        let refund = env::storage_byte_cost().as_yoctonear() * storage_released as u128
            + env::attached_deposit().as_yoctonear();
        Promise::new(account_id.clone()).transfer(NearToken::from_yoctonear(refund));
    }
}

pub(crate) fn assert_at_least_one_yocto() {
    assert!(
        env::attached_deposit() >= ONE_YOCTO,
        "{}",
        errors::NEED_AT_LEAST_ONE_YOCTO
    )
}
