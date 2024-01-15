use crate::{
    assert_at_least_one_yocto, errors, ext_permission_contract, refund_extra_storage_deposit,
    refund_released_storage, Account, BasicPoints, Contract, ContractExt, SubscriptionOutput,
    AFTER_IS_APPROVED_GAS, MAYBE_REFUND_DEPOSIT_GAS, PERMISSION_CONTRACT_GAS,
};
use near_sdk::{
    assert_one_yocto,
    borsh::{BorshDeserialize, BorshSerialize},
    env,
    json_types::{U128, U64},
    near_bindgen,
    serde::{Deserialize, Serialize},
    AccountId, BlockHeight, Duration, Timestamp,
};
use primitive_types::U256;

const MIN_DURATION_BEFORE_START: Duration = 7 * 24 * 60 * 60 * 1_000_000_000;
const MAX_DURATION_BEFORE_START: Duration = 365 * 24 * 60 * 60 * 1_000_000_000;
const MAX_DURATION: Duration = 4 * 366 * 24 * 60 * 60 * 1_000_000_000;
/// Minimum duration. Use 1 nanosecond to run a simple auction.
const MIN_DURATION: Duration = 1;

pub(crate) const MULTIPLIER: u128 = 10u128.pow(38);
pub(crate) const TREASURY_FEE_DENOMINATOR: u128 = 100;
pub(crate) const MAX_NUM_OUT_TOKENS: usize = 4;
pub(crate) const MAX_TITLE_LENGTH: usize = 250;
pub(crate) const MAX_URL_LENGTH: usize = 250;
pub(crate) const MAX_REFERRAL_BPT: u16 = 500;

#[derive(BorshSerialize, BorshDeserialize)]
#[borsh(crate = "near_sdk::borsh")]
pub struct OldSale {
    pub owner_id: AccountId,

    pub title: String,
    pub url: Option<String>,
    pub permissions_contract_id: Option<AccountId>,

    pub out_tokens: Vec<SaleOutToken>,

    pub in_token_account_id: AccountId,
    pub in_token_remaining: u128,
    pub in_token_paid_unclaimed: u128,
    pub in_token_paid: u128,

    pub start_time: Timestamp,
    pub duration: Duration,

    pub total_shares: u128,
    pub last_timestamp: Timestamp,
}

#[derive(BorshSerialize, BorshDeserialize)]
#[borsh(crate = "near_sdk::borsh")]
pub struct Sale {
    pub owner_id: AccountId,

    pub title: String,
    pub url: Option<String>,
    pub permissions_contract_id: Option<AccountId>,

    pub out_tokens: Vec<SaleOutToken>,

    pub in_token_account_id: AccountId,
    pub in_token_remaining: u128,
    pub in_token_paid_unclaimed: u128,
    pub in_token_paid: u128,

    pub start_time: Timestamp,
    pub duration: Duration,

    pub total_shares: u128,
    pub last_timestamp: Timestamp,

    pub start_block_height: BlockHeight,
    pub end_block_height: Option<BlockHeight>,
}

#[derive(BorshSerialize, BorshDeserialize, Clone)]
#[borsh(crate = "near_sdk::borsh")]
pub struct SaleOutToken {
    pub token_account_id: AccountId,
    pub remaining: u128,
    pub distributed: u128,
    pub treasury_unclaimed: Option<u128>,
    pub per_share: [u64; 4],
    pub referral_bpt: Option<BasicPoints>,
}

#[derive(BorshDeserialize, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
pub enum VSale {
    First(OldSale),
    Current(Sale),
}

impl From<Sale> for VSale {
    fn from(sale: Sale) -> Self {
        Self::Current(sale)
    }
}

impl From<VSale> for Sale {
    fn from(v_sale: VSale) -> Self {
        match v_sale {
            VSale::First(old_sale) => {
                let mut sale = Sale {
                    owner_id: old_sale.owner_id,
                    title: old_sale.title,
                    url: old_sale.url,
                    permissions_contract_id: old_sale.permissions_contract_id,
                    out_tokens: old_sale.out_tokens,
                    in_token_account_id: old_sale.in_token_account_id,
                    in_token_remaining: old_sale.in_token_remaining,
                    in_token_paid_unclaimed: old_sale.in_token_paid_unclaimed,
                    in_token_paid: old_sale.in_token_paid,
                    start_time: old_sale.start_time,
                    duration: old_sale.duration,
                    total_shares: old_sale.total_shares,
                    last_timestamp: old_sale.last_timestamp,
                    start_block_height: 0,
                    end_block_height: None,
                };
                sale.touch();
                sale
            }
            VSale::Current(sale) => sale,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct SaleInput {
    pub title: String,
    pub url: Option<String>,
    pub permissions_contract_id: Option<AccountId>,

    pub out_tokens: Vec<SaleInputOutToken>,

    pub in_token_account_id: AccountId,

    pub start_time: U64,
    pub duration: U64,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct SaleInputOutToken {
    pub token_account_id: AccountId,
    pub balance: U128,
    pub referral_bpt: Option<BasicPoints>,
}

impl SaleOutToken {
    pub fn from_input(token: SaleInputOutToken, skyward_token_id: &AccountId) -> Self {
        let is_skyward_token = &token.token_account_id == skyward_token_id;
        Self {
            token_account_id: token.token_account_id,
            remaining: token.balance.into(),
            distributed: 0,
            treasury_unclaimed: if is_skyward_token { None } else { Some(0) },
            per_share: U256::zero().0,
            referral_bpt: token.referral_bpt,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq, Clone))]
pub struct SaleOutput {
    pub sale_id: u64,

    pub title: String,
    pub url: Option<String>,
    pub permissions_contract_id: Option<AccountId>,

    pub owner_id: AccountId,

    pub out_tokens: Vec<SaleOutputOutToken>,

    pub in_token_account_id: AccountId,
    pub in_token_remaining: U128,
    pub in_token_paid_unclaimed: U128,
    pub in_token_paid: U128,

    pub total_shares: U128,

    pub start_time: U64,
    pub duration: U64,
    pub remaining_duration: U64,

    pub subscription: Option<SubscriptionOutput>,

    pub current_time: U64,
    pub current_block_height: U64,
    pub start_block_height: U64,
    pub end_block_height: Option<U64>,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq, Clone))]
pub struct SaleOutputOutToken {
    pub token_account_id: AccountId,
    pub remaining: U128,
    pub distributed: U128,
    pub treasury_unclaimed: Option<U128>,
    pub referral_bpt: Option<BasicPoints>,
}

impl From<SaleOutToken> for SaleOutputOutToken {
    fn from(token: SaleOutToken) -> Self {
        Self {
            token_account_id: token.token_account_id,
            remaining: token.remaining.into(),
            distributed: token.distributed.into(),
            treasury_unclaimed: token.treasury_unclaimed.map(|b| b.into()),
            referral_bpt: token.referral_bpt,
        }
    }
}

impl Sale {
    pub fn touch(&mut self) {
        let end_time = self.start_time + self.duration;
        let timestamp = std::cmp::min(end_time, env::block_timestamp());
        if timestamp <= self.last_timestamp {
            // Sale haven't started or already updated.
            return;
        }
        if self.last_timestamp >= end_time {
            // Sale closed
            return;
        }
        if timestamp >= end_time {
            self.end_block_height = Some(env::block_height());
        }
        if self.total_shares == 0 {
            self.last_timestamp = timestamp;
            return;
        }
        let time_diff = U256::from(timestamp - self.last_timestamp);
        let remaining_duration = U256::from(end_time - self.last_timestamp);

        for out_token in &mut self.out_tokens {
            let mut amount =
                (U256::from(out_token.remaining) * time_diff / remaining_duration).as_u128();
            if amount > 0 {
                out_token.distributed += amount;
                out_token.remaining -= amount;
                if let Some(treasury_unclaimed) = &mut out_token.treasury_unclaimed {
                    let treasury_fee = amount / TREASURY_FEE_DENOMINATOR;
                    *treasury_unclaimed += treasury_fee;
                    amount -= treasury_fee;
                }
                out_token.per_share = (U256(out_token.per_share)
                    + U256::from(amount) * U256::from(MULTIPLIER) / U256::from(self.total_shares))
                .0;
            }
        }

        let in_token_amount =
            (U256::from(self.in_token_remaining) * time_diff / remaining_duration).as_u128();
        self.in_token_paid_unclaimed += in_token_amount;
        self.in_token_paid += in_token_amount;
        self.in_token_remaining -= in_token_amount;

        self.last_timestamp = timestamp;
    }

    pub fn assert_valid_not_started(&self) {
        let timestamp = env::block_timestamp();
        assert!(
            self.owner_id == env::current_account_id()
                || self.start_time >= timestamp + MIN_DURATION_BEFORE_START,
            "{}",
            errors::STARTS_TOO_SOON
        );
        assert!(
            self.start_time < timestamp + MAX_DURATION_BEFORE_START,
            "{}",
            errors::MAX_DURATION_TO_START
        );
        assert!(self.duration <= MAX_DURATION, "{}", errors::MAX_DURATION);
        assert!(self.duration >= MIN_DURATION, "{}", errors::MIN_DURATION);
        assert!(
            self.out_tokens.len() <= MAX_NUM_OUT_TOKENS,
            "{}",
            errors::MAX_NUM_OUT_TOKENS
        );
        assert!(
            self.title.len() <= MAX_TITLE_LENGTH,
            "{}",
            errors::TOO_LONG_TITLE
        );
        assert!(
            self.url.as_ref().map(|s| s.len()).unwrap_or(0) <= MAX_URL_LENGTH,
            "{}",
            errors::TOO_LONG_URL
        );

        let mut unique_tokens = Vec::with_capacity(self.out_tokens.len());
        for out_token in &self.out_tokens {
            assert!(out_token.remaining > 0, "{}", errors::ZERO_OUT_AMOUNT);
            assert_ne!(
                self.in_token_account_id,
                out_token.token_account_id,
                "{}",
                errors::SAME_TOKENS
            );
            if let Some(referral_bpt) = out_token.referral_bpt {
                assert!(
                    referral_bpt <= MAX_REFERRAL_BPT,
                    "{}",
                    errors::MAX_REFERRAL_BPT
                );
            }
            unique_tokens.push(out_token.token_account_id.clone());
        }
        unique_tokens.sort();
        unique_tokens.dedup();
        assert_eq!(
            unique_tokens.len(),
            self.out_tokens.len(),
            "{}",
            errors::NON_UNIQUE_OUT_TOKENS
        );
    }

    pub fn from_input(sale: SaleInput, owner_id: AccountId, skyward_token_id: &AccountId) -> Self {
        let start_time = sale.start_time.0;
        Sale {
            owner_id,
            title: sale.title,
            url: sale.url,
            permissions_contract_id: sale.permissions_contract_id,
            out_tokens: sale
                .out_tokens
                .into_iter()
                .map(|o| SaleOutToken::from_input(o, skyward_token_id))
                .collect(),
            in_token_account_id: sale.in_token_account_id,
            in_token_remaining: 0,
            in_token_paid_unclaimed: 0,
            in_token_paid: 0,
            total_shares: 0,
            start_time,
            duration: sale.duration.0,
            last_timestamp: start_time,
            start_block_height: env::block_height(),
            end_block_height: None,
        }
    }

    pub fn into_output(self, sale_id: u64, account: Option<&Account>) -> SaleOutput {
        let remaining_duration = self.start_time + self.duration - self.last_timestamp;
        let subscription =
            account.and_then(|account| account.internal_subscription_output(sale_id, &self));
        SaleOutput {
            sale_id,
            owner_id: self.owner_id,
            title: self.title,
            url: self.url,
            permissions_contract_id: self.permissions_contract_id,
            out_tokens: self.out_tokens.into_iter().map(|o| o.into()).collect(),
            in_token_account_id: self.in_token_account_id,
            in_token_remaining: self.in_token_remaining.into(),
            in_token_paid_unclaimed: self.in_token_paid_unclaimed.into(),
            in_token_paid: self.in_token_paid.into(),
            total_shares: self.total_shares.into(),
            start_time: self.start_time.into(),
            duration: self.duration.into(),
            remaining_duration: remaining_duration.into(),
            subscription,
            current_time: env::block_timestamp().into(),
            current_block_height: env::block_height().into(),
            start_block_height: self.start_block_height.into(),
            end_block_height: self.end_block_height.map(|height| height.into()),
        }
    }

    /// Returns remaining in_balance
    pub fn shares_to_in_balance(&self, shares: u128) -> u128 {
        if shares == 0 {
            return 0;
        }
        (U256::from(self.in_token_remaining) * U256::from(shares) / U256::from(self.total_shares))
            .as_u128()
    }

    pub fn in_amount_to_shares(&self, in_amount: u128, round_up: bool) -> u128 {
        if self.total_shares == 0 {
            return in_amount;
        }
        assert!(
            self.in_token_remaining > 0 && !self.has_ended(),
            "{}",
            errors::SALE_ENDED
        );
        let in_token_remaining = U256::from(self.in_token_remaining);
        let num_shares = U256::from(in_amount) * U256::from(self.total_shares);
        let num_shares = if round_up {
            (num_shares + in_token_remaining - 1) / in_token_remaining
        } else {
            num_shares / in_token_remaining
        };
        if !round_up {
            assert!(
                num_shares + U256::from(self.total_shares) < U256::from(u128::MAX),
                "{}",
                errors::SHARES_OVERFLOW
            );
        }
        num_shares.as_u128()
    }

    pub fn has_ended(&self) -> bool {
        self.last_timestamp >= self.start_time + self.duration
    }
}

impl Contract {
    pub fn internal_unwrap_sale(&self, sale_id: u64) -> Sale {
        self.sales
            .get(&sale_id)
            .expect(errors::SALE_NOT_FOUND)
            .into()
    }

    pub fn internal_get_sale(&self, sale_id: u64, account: Option<&Account>) -> Option<SaleOutput> {
        self.sales.get(&sale_id).map(|v_sale| {
            let sale: Sale = v_sale.into();
            sale.into_output(sale_id, account)
        })
    }

    pub fn internal_distribute_unclaimed_tokens(&mut self, sale: &mut Sale) {
        if sale.in_token_paid_unclaimed > 0 {
            if sale.owner_id == env::current_account_id() {
                // Skyward Sale
                self.treasury
                    .internal_donate(&sale.in_token_account_id, sale.in_token_paid_unclaimed);
            } else {
                let mut account = self.internal_unwrap_account(&sale.owner_id);
                if sale.in_token_account_id != self.treasury.skyward_token_id {
                    let treasury_fee = sale.in_token_paid_unclaimed / TREASURY_FEE_DENOMINATOR;
                    self.treasury
                        .internal_deposit(&sale.in_token_account_id, treasury_fee);
                    sale.in_token_paid_unclaimed -= treasury_fee;
                }
                account.internal_token_deposit(
                    &sale.in_token_account_id,
                    sale.in_token_paid_unclaimed,
                );
                self.accounts.insert(&sale.owner_id, &account.into());
            }

            sale.in_token_paid_unclaimed = 0;
        }
        let sale_ended = sale.has_ended();
        for out_token in &mut sale.out_tokens {
            if let Some(treasury_unclaimed) = &mut out_token.treasury_unclaimed {
                self.treasury
                    .internal_deposit(&out_token.token_account_id, *treasury_unclaimed);
                *treasury_unclaimed = 0;
            }
            if sale_ended && out_token.remaining > 0 {
                // No one subscribed at the end of the sale
                if sale.owner_id == env::current_account_id() {
                    self.treasury
                        .internal_donate(&out_token.token_account_id, out_token.remaining);
                } else {
                    let mut account = self.internal_unwrap_account(&sale.owner_id);
                    account
                        .internal_token_deposit(&out_token.token_account_id, out_token.remaining);
                    self.accounts.insert(&sale.owner_id, &account.into());
                }
                out_token.distributed += out_token.remaining;
                out_token.remaining = 0;
            }
        }
    }
}

#[near_bindgen]
impl Contract {
    #[payable]
    pub fn sale_create(&mut self, sale: SaleInput) -> u64 {
        let initial_storage_usage = env::storage_usage();
        let sale_id = self.num_sales;
        let sale = Sale::from_input(
            sale,
            env::predecessor_account_id(),
            &self.treasury.skyward_token_id,
        );
        sale.assert_valid_not_started();

        let mut account = self.internal_unwrap_account(&sale.owner_id);
        for out_token in &sale.out_tokens {
            if out_token.remaining > 0 {
                account.internal_token_withdraw(&out_token.token_account_id, out_token.remaining);
            }
        }
        self.internal_maybe_register_token(&mut account, &sale.in_token_account_id);
        account.sales.insert(&sale_id);

        self.accounts.insert(&sale.owner_id, &account.into());
        self.sales.insert(&sale_id, &sale.into());
        self.num_sales += 1;

        refund_extra_storage_deposit(
            env::storage_usage() - initial_storage_usage,
            self.treasury.listing_fee_near,
        );
        sale_id
    }

    pub fn get_sale(&self, sale_id: u64, account_id: Option<AccountId>) -> Option<SaleOutput> {
        let account: Option<Account> =
            account_id.and_then(|account_id| self.accounts.get(&account_id).map(|a| a.into()));
        self.internal_get_sale(sale_id, account.as_ref())
    }

    pub fn get_sales(
        &self,
        account_id: Option<AccountId>,
        from_index: Option<u64>,
        limit: Option<u64>,
    ) -> Vec<SaleOutput> {
        let account: Option<Account> =
            account_id.and_then(|account_id| self.accounts.get(&account_id).map(|a| a.into()));
        let from_index = from_index.unwrap_or(0);
        let limit = limit.unwrap_or(self.num_sales);
        (from_index..std::cmp::min(from_index + limit, self.num_sales))
            .filter_map(|sale_id| self.internal_get_sale(sale_id, account.as_ref()))
            .collect()
    }

    pub fn get_sales_by_id(
        &self,
        account_id: Option<AccountId>,
        sale_ids: Vec<u64>,
    ) -> Vec<SaleOutput> {
        let account: Option<Account> =
            account_id.and_then(|account_id| self.accounts.get(&account_id).map(|a| a.into()));
        sale_ids
            .into_iter()
            .filter_map(|sale_id| self.internal_get_sale(sale_id, account.as_ref()))
            .collect()
    }

    #[payable]
    pub fn sale_deposit_in_token(
        &mut self,
        sale_id: u64,
        amount: U128,
        referral_id: Option<AccountId>,
    ) {
        assert_at_least_one_yocto();
        let initial_storage_usage = env::storage_usage();
        let account_id = env::predecessor_account_id();

        let in_amount = amount.0;

        let permissions_contract_id = self.internal_deposit_in_amount(
            sale_id,
            &account_id,
            in_amount,
            referral_id.as_ref(),
            false,
        );

        if let Some(permissions_contract_id) = permissions_contract_id {
            let attached_deposit = env::attached_deposit();
            self.treasury.locked_attached_deposits += env::attached_deposit().as_yoctonear();
            ext_permission_contract::ext(permissions_contract_id)
                .with_static_gas(PERMISSION_CONTRACT_GAS)
                .is_approved(account_id.clone(), sale_id)
                .then(
                    Self::ext(env::current_account_id())
                        .with_static_gas(AFTER_IS_APPROVED_GAS)
                        .after_is_approved(
                            sale_id,
                            account_id.clone(),
                            in_amount.into(),
                            referral_id,
                            attached_deposit.as_yoctonear().into(),
                        ),
                )
                .then(
                    Self::ext(env::current_account_id())
                        .with_static_gas(MAYBE_REFUND_DEPOSIT_GAS)
                        .maybe_refund_deposit(
                            account_id.clone(),
                            attached_deposit.as_yoctonear().into(),
                        ),
                )
                .as_return();
        } else {
            refund_extra_storage_deposit(env::storage_usage() - initial_storage_usage, 0);
        }
    }

    #[payable]
    pub fn sale_withdraw_in_token(&mut self, sale_id: u64, shares: Option<U128>) {
        assert_one_yocto();
        let initial_storage_usage = env::storage_usage();
        let account_id = env::predecessor_account_id();
        self.internal_withdraw_shares(sale_id, &account_id, shares.map(|s| s.0));
        refund_released_storage(&account_id, initial_storage_usage - env::storage_usage());
    }

    #[payable]
    pub fn sale_withdraw_in_token_exact(&mut self, sale_id: u64, amount: U128) {
        assert_one_yocto();
        let initial_storage_usage = env::storage_usage();
        let account_id = env::predecessor_account_id();
        self.internal_withdraw_in_token_exact(sale_id, &account_id, amount.0);
        refund_released_storage(&account_id, initial_storage_usage - env::storage_usage());
    }

    /// This method can be called by anyone in order to move in tokens to treasury
    pub fn sale_distribute_unclaimed_tokens(&mut self, sale_id: u64) {
        let mut sale = self.internal_unwrap_sale(sale_id);
        self.internal_distribute_unclaimed_tokens(&mut sale);
        self.sales.insert(&sale_id, &sale.into());
    }

    pub fn sale_claim_out_tokens(&mut self, sale_id: u64) {
        let account_id = env::predecessor_account_id();
        let initial_storage_usage = env::storage_usage();
        let mut sale = self.internal_unwrap_sale(sale_id);
        self.internal_distribute_unclaimed_tokens(&mut sale);
        let mut account = self.internal_unwrap_account(&account_id);
        let subscription =
            self.internal_update_subscription(&mut account, sale_id, &mut sale, None, false);

        account.internal_save_subscription(sale_id, &sale, subscription);

        self.accounts.insert(&account_id, &account.into());
        self.sales.insert(&sale_id, &sale.into());
        refund_released_storage(&account_id, initial_storage_usage - env::storage_usage());
    }
}
