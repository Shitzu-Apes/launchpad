use crate::{
    assert_at_least_one_yocto, errors, refund_extra_storage_deposit, Contract, ContractExt,
    FtOnTransferArgs, Sale, SaleOutput, StorageKey, Subscription, SubscriptionOutput,
    VSubscription,
};
use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    collections::{UnorderedMap, UnorderedSet},
    env,
    json_types::U128,
    near_bindgen, serde_json, AccountId, Promise, PromiseOrValue,
};
use primitive_types::U256;

const REFERRAL_FEE_DENOMINATOR: u128 = 10000;

#[derive(BorshSerialize, BorshDeserialize)]
#[borsh(crate = "near_sdk::borsh")]
pub struct Account {
    pub balances: UnorderedMap<AccountId, u128>,
    pub subs: UnorderedMap<u64, VSubscription>,
    pub sales: UnorderedSet<u64>,
}

#[derive(BorshDeserialize, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
pub enum VAccount {
    Current(Account),
}

impl From<Account> for VAccount {
    fn from(account: Account) -> Self {
        Self::Current(account)
    }
}

impl From<VAccount> for Account {
    fn from(v_account: VAccount) -> Self {
        match v_account {
            VAccount::Current(account) => account,
        }
    }
}

impl Account {
    pub fn internal_token_deposit(&mut self, token_account_id: &AccountId, amount: u128) {
        let balance = self
            .balances
            .get(token_account_id)
            .expect(errors::TOKEN_NOT_REGISTERED);
        let new_balance = balance.checked_add(amount).expect(errors::BALANCE_OVERFLOW);
        self.balances.insert(token_account_id, &new_balance);
    }

    pub fn internal_token_withdraw(&mut self, token_account_id: &AccountId, amount: u128) {
        let balance = self
            .balances
            .get(token_account_id)
            .expect(errors::TOKEN_NOT_REGISTERED);
        let new_balance = balance
            .checked_sub(amount)
            .expect(errors::NOT_ENOUGH_BALANCE);
        self.balances.insert(token_account_id, &new_balance);
    }

    pub fn internal_get_subscription(
        &self,
        sale_id: u64,
        sale: &Sale,
        referral_id: Option<&AccountId>,
        create_new: bool,
    ) -> (Subscription, Vec<u128>) {
        let mut subscription: Subscription = self
            .subs
            .get(&sale_id)
            .map(|s| s.into())
            .unwrap_or_else(|| {
                if create_new {
                    Subscription::new(sale, referral_id.cloned())
                } else {
                    env::panic_str(errors::NO_PERMISSION)
                }
            });
        let out_token_amounts = subscription.touch(sale);
        (subscription, out_token_amounts)
    }

    pub fn internal_save_subscription(
        &mut self,
        sale_id: u64,
        sale: &Sale,
        subscription: Subscription,
    ) {
        if subscription.shares == 0 && (sale.permissions_contract_id.is_none() || sale.has_ended())
        {
            self.subs.remove(&sale_id);
        } else {
            self.subs.insert(&sale_id, &subscription.into());
        }
    }

    pub fn internal_subscription_output(
        &self,
        sale_id: u64,
        sale: &Sale,
    ) -> Option<SubscriptionOutput> {
        let (subscription, out_token_remaining) =
            self.internal_get_subscription(sale_id, sale, None, true);
        if subscription.shares > 0 || out_token_remaining.iter().any(|&v| v > 0) {
            let remaining_in_balance = sale.shares_to_in_balance(subscription.shares);
            Some(SubscriptionOutput {
                remaining_in_balance: remaining_in_balance.into(),
                spent_in_balance: (subscription.spent_in_balance_without_shares
                    + (subscription.last_in_balance - remaining_in_balance))
                    .into(),
                unclaimed_out_balances: out_token_remaining.into_iter().map(|b| b.into()).collect(),
                claimed_out_balance: subscription
                    .claimed_out_balance
                    .into_iter()
                    .map(|b| b.into())
                    .collect(),
                shares: subscription.shares.into(),
                referral_id: subscription.referral_id,
            })
        } else {
            None
        }
    }
}

impl Contract {
    pub fn internal_unwrap_account(&self, account_id: &AccountId) -> Account {
        self.accounts
            .get(account_id)
            .expect(errors::ACCOUNT_NOT_FOUND)
            .into()
    }

    pub fn internal_maybe_register_token(
        &mut self,
        account: &mut Account,
        token_account_id: &AccountId,
    ) {
        if account.balances.get(token_account_id).is_none() {
            account.balances.insert(token_account_id, &0);
            self.treasury.internal_deposit(token_account_id, 0);
        }
    }

    pub fn internal_update_subscription(
        &mut self,
        account: &mut Account,
        sale_id: u64,
        sale: &mut Sale,
        referral_id: Option<&AccountId>,
        passed_permission_check: bool,
    ) -> Subscription {
        let create_new = passed_permission_check || sale.permissions_contract_id.is_none();
        let (mut subscription, out_token_amounts) =
            account.internal_get_subscription(sale_id, sale, referral_id, create_new);
        for (index, (mut amount, out_token)) in out_token_amounts
            .into_iter()
            .zip(sale.out_tokens.iter())
            .enumerate()
        {
            if amount > 0 {
                if let Some(referral_bpt) = out_token.referral_bpt {
                    let mut ref_amount = (U256::from(amount) * U256::from(referral_bpt)
                        / U256::from(REFERRAL_FEE_DENOMINATOR))
                    .as_u128();
                    let referral_id = subscription
                        .referral_id
                        .as_ref()
                        .map(|referral_id| {
                            ref_amount /= 2;
                            referral_id
                        })
                        .unwrap_or(&sale.owner_id);
                    if ref_amount > 0 {
                        amount -= ref_amount;
                        if let Some(referral) = self.accounts.get(referral_id) {
                            let mut referral: Account = referral.into();
                            if referral.balances.get(&out_token.token_account_id).is_some() {
                                referral.internal_token_deposit(
                                    &out_token.token_account_id,
                                    ref_amount,
                                );
                                ref_amount = 0;
                                self.accounts.insert(referral_id, &referral.into());
                            }
                        }
                        if ref_amount > 0 {
                            self.treasury
                                .internal_deposit(&out_token.token_account_id, ref_amount);
                        }
                    }
                }
                account.internal_token_deposit(&out_token.token_account_id, amount);
                subscription.claimed_out_balance[index] += amount;
            }
        }
        if subscription.shares > 0 {
            let remaining_in_amount = sale.shares_to_in_balance(subscription.shares);
            if remaining_in_amount == 0 {
                sale.total_shares -= subscription.shares;
                subscription.shares = 0;
            }
        }
        subscription
    }
}

#[near_bindgen]
impl Contract {
    #[payable]
    pub fn register_token(&mut self, account_id: Option<AccountId>, token_account_id: AccountId) {
        self.register_tokens(account_id, vec![token_account_id])
    }

    #[payable]
    pub fn register_tokens(
        &mut self,
        account_id: Option<AccountId>,
        token_account_ids: Vec<AccountId>,
    ) {
        assert_at_least_one_yocto();
        let initial_storage_usage = env::storage_usage();
        let account_id = account_id.unwrap_or_else(env::predecessor_account_id);
        let mut account = self
            .accounts
            .get(&account_id)
            .map(|a| a.into())
            .unwrap_or_else(|| Account {
                balances: UnorderedMap::new(StorageKey::AccountTokens {
                    account_id: account_id.clone(),
                }),
                subs: UnorderedMap::new(StorageKey::AccountSubs {
                    account_id: account_id.clone(),
                }),
                sales: UnorderedSet::new(StorageKey::AccountSales {
                    account_id: account_id.clone(),
                }),
            });
        for token_account_id in token_account_ids {
            self.internal_maybe_register_token(&mut account, &token_account_id);
        }
        self.accounts.insert(&account_id, &account.into());
        refund_extra_storage_deposit(env::storage_usage() - initial_storage_usage, 0);
    }

    pub fn withdraw_token(&mut self, token_account_id: AccountId, amount: Option<U128>) -> Promise {
        let account_id = env::predecessor_account_id();
        let mut account = self.internal_unwrap_account(&account_id);
        let amount = amount.map(|a| a.0).unwrap_or_else(|| {
            account
                .balances
                .get(&token_account_id)
                .expect(errors::TOKEN_NOT_REGISTERED)
        });
        account.internal_token_withdraw(&token_account_id, amount);
        self.internal_ft_transfer(&account_id, &token_account_id, amount)
    }

    pub fn balance_of(&self, account_id: AccountId, token_account_id: AccountId) -> Option<U128> {
        self.accounts.get(&account_id).and_then(|account| {
            let account: Account = account.into();
            account.balances.get(&token_account_id).map(|a| a.into())
        })
    }

    pub fn balances_of(
        &self,
        account_id: AccountId,
        from_index: Option<u64>,
        limit: Option<u64>,
    ) -> Vec<(AccountId, U128)> {
        if let Some(account) = self.accounts.get(&account_id) {
            let account: Account = account.into();
            let keys = account.balances.keys_as_vector();
            let values = account.balances.values_as_vector();
            let from_index = from_index.unwrap_or(0);
            let limit = limit.unwrap_or(keys.len());
            (from_index..std::cmp::min(from_index + limit, keys.len()))
                .map(|index| (keys.get(index).unwrap(), values.get(index).unwrap().into()))
                .collect()
        } else {
            vec![]
        }
    }

    pub fn get_num_balances(&self, account_id: AccountId) -> u64 {
        self.accounts
            .get(&account_id)
            .map(|account| {
                let account: Account = account.into();
                account.balances.len()
            })
            .unwrap_or(0)
    }

    pub fn get_subscribed_sales(
        &self,
        account_id: AccountId,
        from_index: Option<u64>,
        limit: Option<u64>,
    ) -> Vec<SaleOutput> {
        if let Some(account) = self.accounts.get(&account_id) {
            let account: Account = account.into();
            let keys = account.subs.keys_as_vector();
            let from_index = from_index.unwrap_or(0);
            let limit = limit.unwrap_or(keys.len());
            (from_index..std::cmp::min(from_index + limit, keys.len()))
                .filter_map(|index| {
                    let sale_id = keys.get(index).unwrap();
                    self.internal_get_sale(sale_id, Some(&account))
                })
                .collect()
        } else {
            vec![]
        }
    }

    pub fn get_account_sales(
        &self,
        account_id: AccountId,
        from_index: Option<u64>,
        limit: Option<u64>,
    ) -> Vec<SaleOutput> {
        if let Some(account) = self.accounts.get(&account_id) {
            let account: Account = account.into();
            let keys = account.sales.as_vector();
            let from_index = from_index.unwrap_or(0);
            let limit = limit.unwrap_or(keys.len());
            (from_index..std::cmp::min(from_index + limit, keys.len()))
                .filter_map(|index| {
                    let sale_id = keys.get(index).unwrap();
                    self.internal_get_sale(sale_id, Some(&account))
                })
                .collect()
        } else {
            vec![]
        }
    }
}

#[near_bindgen]
impl FungibleTokenReceiver for Contract {
    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        let args: FtOnTransferArgs =
            serde_json::from_str(&msg).expect(errors::FAILED_TO_PARSE_FT_ON_TRANSFER_MSG);
        let token_account_id = env::predecessor_account_id();
        match args {
            FtOnTransferArgs::AccountDeposit => {
                let mut account = self.internal_unwrap_account(&sender_id);
                account.internal_token_deposit(&token_account_id, amount.0);
            }
        }
        PromiseOrValue::Value(0.into())
    }
}
