pub mod event;

use super::*;
use near_sdk::Timestamp;
use near_workspaces::{
    network::Sandbox,
    result::{ExecutionFinalResult, ExecutionResult, Value},
    Account, Contract, Worker,
};
use owo_colors::OwoColorize;

#[macro_export]
macro_rules! print_log {
    ( $x:expr, $($y:expr),+ ) => {
        let thread_name = std::thread::current().name().unwrap().to_string();
        if thread_name == "main" {
            println!($x, $($y),+);
        } else {
            let mut s = format!($x, $($y),+);
            s = s.split('\n').map(|s| {
                let mut pre = "    ".to_string();
                pre.push_str(s);
                pre.push('\n');
                pre
            }).collect::<String>();
            println!(
                "{}\n{}",
                thread_name.bold(),
                &s[..s.len() - 1],
            );
        }
    };
}

pub fn log_tx_result(
    ident: &str,
    res: ExecutionFinalResult,
) -> anyhow::Result<(ExecutionResult<Value>, Vec<event::ContractEvent>)> {
    for failure in res.receipt_failures() {
        print_log!("{:#?}", failure.bright_red());
    }
    let mut events = vec![];
    for outcome in res.receipt_outcomes() {
        if !outcome.logs.is_empty() {
            for log in outcome.logs.iter() {
                if log.starts_with("EVENT_JSON:") {
                    let event: event::ContractEvent =
                        serde_json::from_str(&log.replace("EVENT_JSON:", ""))?;
                    events.push(event.clone());
                    print_log!(
                        "{}: {}\n{}",
                        "account".bright_cyan(),
                        outcome.executor_id,
                        event
                    );
                } else {
                    print_log!("{}", log.bright_yellow());
                }
            }
        }
    }
    print_log!(
        "{} gas burnt: {:.3} {}",
        ident.italic(),
        res.total_gas_burnt.as_tgas().bright_magenta().bold(),
        "TGas".bright_magenta().bold()
    );
    Ok((res.into_result()?, events))
}

pub struct Env {
    pub worker: Worker<Sandbox>,
    pub skyward_dao: Account,
    pub skyward: Contract,
    pub skyward_token: Contract,
    pub permissions_contract: Contract,
    pub w_near: Contract,

    pub users: Vec<Account>,
}

pub fn to_nano(timestamp: u32) -> Timestamp {
    Timestamp::from(timestamp) * 10u64.pow(9)
}

impl Env {
    pub async fn init(num_users: usize) -> anyhow::Result<Self> {
        Self::init_with_schedule(num_users).await
    }

    pub async fn init_with_schedule(num_users: usize) -> anyhow::Result<Self> {
        let worker = near_workspaces::sandbox().await?;
        let skyward_dao = worker
            .create_tla(
                SKYWARD_DAO_ID.parse()?,
                SecretKey::from_random(KeyType::ED25519),
            )
            .await?
            .into_result()?;
        let w_near = worker
            .create_tla_and_deploy(
                WRAP_NEAR_ID.parse()?,
                SecretKey::from_random(KeyType::ED25519),
                W_NEAR_WASM_BYTES,
            )
            .await?
            .into_result()?;
        log_tx_result(
            "Initialize wNEAR contract",
            w_near.call("new").transact().await?,
        )?;
        let permissions_contract = worker
            .create_tla_and_deploy(
                PERMISSIONS_CONTRACT_ID.parse()?,
                SecretKey::from_random(KeyType::ED25519),
                PERMISSIONS_WASM_BYTES,
            )
            .await?
            .into_result()?;
        log_tx_result(
            "Initialize permissions contract",
            permissions_contract
                .call("new")
                .args_json(json!({
                    "owner_id": skyward_dao.id(),
                }))
                .transact()
                .await?,
        )?;
        let skyward = worker
            .create_tla_and_deploy(
                SKYWARD_ID.parse()?,
                SecretKey::from_random(KeyType::ED25519),
                SKYWARD_WASM_BYTES,
            )
            .await?
            .into_result()?;
        log_tx_result(
            "Initialize Skyward contract",
            skyward
                .call("new")
                .args_json((
                    SKYWARD_DAO_ID,
                    U128(LISTING_FEE_NEAR.as_yoctonear()),
                    w_near.id(),
                ))
                .transact()
                .await?,
        )?;
        let skyward_token = worker
            .create_tla_and_deploy(
                SKYWARD_TOKEN_ID.parse()?,
                SecretKey::from_random(KeyType::ED25519),
                FUNGIBLE_TOKEN_WASM_BYTES,
            )
            .await?
            .into_result()?;
        log_tx_result(
            "Initialize Skyward token contract",
            skyward_token
                .call("new")
                .args_json(json!({
                    "owner_id": skyward_dao.id(),
                    "total_supply": U128(SKYWARD_TOTAL_SUPPLY),
                    "metadata": FungibleTokenMetadata {
                        spec: FT_METADATA_SPEC.to_string(),
                        name: "Skyward Finance Token".to_string(),
                        symbol: "SKYWARD".to_string(),
                        icon: None,
                        reference: None,
                        reference_hash: None,
                        decimals: SKYWARD_TOKEN_DECIMALS,
                    }
                }))
                .transact()
                .await?,
        )?;
        let mut this = Self {
            worker,
            skyward_dao,
            skyward,
            skyward_token,
            permissions_contract,
            w_near,
            users: vec![],
        };
        this.storage_deposit(&this.w_near, &this.skyward_dao, None, None)
            .await?;
        this.storage_deposit(&this.w_near, this.skyward.as_account(), None, None)
            .await?;
        this.init_users(num_users).await?;
        Ok(this)
    }

    pub async fn storage_deposit(
        &self,
        contract: &Contract,
        sender: &Account,
        account_id: Option<&AccountId>,
        deposit: Option<NearToken>,
    ) -> anyhow::Result<ExecutionResult<Value>> {
        let (res, _) = log_tx_result(
            &format!(
                "Sender {} calling 'storage_deposit' on account {} for account {}",
                sender.id(),
                contract.id(),
                if let Some(account_id) = account_id {
                    account_id.as_str()
                } else {
                    "self"
                }
            ),
            sender
                .call(contract.id(), "storage_deposit")
                .args_json((account_id, None::<bool>))
                .deposit(deposit.unwrap_or(NearToken::from_millinear(50)))
                .max_gas()
                .transact()
                .await?,
        )?;
        Ok(res)
    }

    pub async fn deploy_ft(
        &self,
        owner_id: &AccountId,
        token_account_id: &str,
    ) -> anyhow::Result<Contract> {
        let token = self
            .worker
            .create_tla_and_deploy(
                token_account_id.parse()?,
                SecretKey::from_random(KeyType::ED25519),
                FUNGIBLE_TOKEN_WASM_BYTES,
            )
            .await?
            .into_result()?;
        token
            .call("new_default_meta")
            .args_json(json!({
                "owner_id": owner_id,
                "total_supply": U128::from(DEFAULT_TOTAL_SUPPLY)
            }))
            .transact()
            .await?
            .into_result()?;
        self.storage_deposit(&token, self.skyward.as_account(), None, None)
            .await?;
        self.storage_deposit(&token, &self.skyward_dao, None, None)
            .await?;
        Ok(token)
    }

    pub async fn wrap_near(&self, user: &Account, amount: NearToken) -> anyhow::Result<()> {
        user.call(self.w_near.id(), "near_deposit")
            .args_json(&json!({
                "account_id": user.id()
            }))
            .deposit(amount)
            .transact()
            .await?
            .into_result()?;
        Ok(())
    }

    pub async fn register_and_deposit(
        &self,
        user: &Account,
        token_id: &AccountId,
        amount: NearToken,
    ) -> anyhow::Result<()> {
        log_tx_result(
            "register_token",
            user.call(self.skyward.id(), "register_token")
                .args_json((None::<AccountId>, token_id))
                .deposit(NearToken::from_millinear(10))
                .transact()
                .await?,
        )?;

        log_tx_result(
            "ft_transfer_call",
            user.call(token_id, "ft_transfer_call")
                .args_json(json!({
                    "receiver_id": self.skyward.id(),
                    "amount": U128::from(amount.as_yoctonear()),
                    "msg": "\"AccountDeposit\""
                }))
                .max_gas()
                .deposit(NearToken::from_yoctonear(1))
                .transact()
                .await?,
        )?;

        Ok(())
    }

    pub async fn init_users(&mut self, num_users: usize) -> anyhow::Result<()> {
        for _ in 0..num_users {
            let user = self.worker.dev_create_account().await?;
            self.wrap_near(&user, NearToken::from_near(20)).await?;
            self.register_and_deposit(&user, self.w_near.id(), NearToken::from_near(10))
                .await?;
            self.users.push(user);
        }
        Ok(())
    }

    pub async fn sale_create(
        &self,
        user: &Account,
        tokens: &[(&Account, u128)],
        start_time: u64,
    ) -> anyhow::Result<SaleOutput> {
        self.sale_create_custom(user, tokens, start_time, BLOCK_DURATION * 60, None, None)
            .await
    }

    pub async fn sale_create_with_ref(
        &self,
        user: &Account,
        tokens: &[(&Account, u128)],
        start_time: u64,
    ) -> anyhow::Result<SaleOutput> {
        self.sale_create_custom(
            user,
            tokens,
            start_time,
            BLOCK_DURATION * 60,
            None,
            Some(100),
        )
        .await
    }

    pub async fn sale_create_custom(
        &self,
        user: &Account,
        tokens: &[(&Account, u128)],
        start_time: u64,
        sale_duration: u64,
        permissions_contract_id: Option<AccountId>,
        referral_bpt: Option<u16>,
    ) -> anyhow::Result<SaleOutput> {
        let initial_balance = user.view_account().await?.balance;

        let deposit = if user.id().as_str() != SKYWARD_ID {
            NearToken::from_near(1)
                .checked_add(LISTING_FEE_NEAR)
                .unwrap()
        } else {
            NearToken::from_yoctonear(0)
        };
        let sale_id: u64 = log_tx_result(
            "sale_create",
            user.call(self.skyward.id(), "sale_create")
                .args_json((SaleInput {
                    title: TITLE.to_string(),
                    url: None,
                    permissions_contract_id: permissions_contract_id.map(|id| id.parse().unwrap()),
                    out_tokens: tokens
                        .iter()
                        .map(|(token, balance)| SaleInputOutToken {
                            token_account_id: token.id().parse().unwrap(),
                            balance: (*balance).into(),
                            referral_bpt,
                        })
                        .collect(),
                    in_token_account_id: self.w_near.id().parse()?,
                    start_time: start_time.into(),
                    duration: sale_duration.into(),
                },))
                .deposit(deposit)
                .transact()
                .await?,
        )?
        .0
        .json()?;

        let balance_spent = initial_balance
            .checked_sub(user.view_account().await?.balance)
            .unwrap();
        // Should be listing fee plus some for storage. The rest should be refunded.
        assert!(
            LISTING_FEE_NEAR < balance_spent
                && balance_spent
                    < LISTING_FEE_NEAR
                        .checked_add(NearToken::from_millinear(20))
                        .unwrap()
        );

        self.get_sale(sale_id, None).await
    }

    pub async fn withdraw_token(
        &self,
        user: &Account,
        token_id: &AccountId,
        amount: Option<u128>,
    ) -> anyhow::Result<()> {
        log_tx_result(
            "withdraw_token",
            user.call(self.skyward.id(), "withdraw_token")
                .args_json((token_id, amount.map(U128)))
                .gas(Gas::from_tgas(50))
                .transact()
                .await?,
        )?;
        Ok(())
    }

    pub async fn get_sale(
        &self,
        sale_id: u64,
        account_id: Option<AccountId>,
    ) -> anyhow::Result<SaleOutput> {
        Ok(self
            .worker
            .view(self.skyward.id(), "get_sale")
            .args_json((sale_id, account_id))
            .await?
            .json()?)
    }

    pub async fn balances_of(&self, user: &Account) -> anyhow::Result<Vec<(AccountId, u128)>> {
        let res: Vec<(AccountId, U128)> = user
            .view(self.skyward.id(), "balances_of")
            .args_json((user.id(), None::<u64>, None::<u64>))
            .await?
            .json()?;
        Ok(res.into_iter().map(|(a, b)| (a, b.0)).collect())
    }

    pub async fn ft_balance_of(
        &self,
        user: &Account,
        token_id: &AccountId,
    ) -> anyhow::Result<u128> {
        let res: U128 = user
            .view(token_id, "ft_balance_of")
            .args_json((user.id(),))
            .await?
            .json()?;
        Ok(res.0)
    }

    pub async fn claim_treasury(&self, user: &Account) -> anyhow::Result<()> {
        log_tx_result(
            "claim_treasury",
            user.call(self.skyward.id(), "claim_treasury")
                .max_gas()
                .transact()
                .await?,
        )?;
        Ok(())
    }

    pub async fn get_treasury_balances(&self) -> anyhow::Result<Vec<(AccountId, u128)>> {
        let res: Vec<(AccountId, U128)> = self
            .worker
            .view(self.skyward.id(), "get_treasury_balances")
            .args_json(json!({}))
            .await?
            .json()?;
        Ok(res.into_iter().map(|(a, b)| (a, b.0)).collect())
    }

    pub async fn get_token_balance(
        &self,
        token_id: &AccountId,
        user: &Account,
    ) -> anyhow::Result<u128> {
        let balance: U128 = self
            .worker
            .view(token_id, "ft_balance_of")
            .args_json(json!({
                "account_id": user.id(),
            }))
            .await?
            .json()?;
        Ok(balance.0)
    }

    pub fn assert_sale_eq(&self, sale: &SaleOutput, expected: PartialSale) {
        assert_eq!(
            sale,
            &SaleOutput {
                out_tokens: expected
                    .out_tokens
                    .into_iter()
                    .enumerate()
                    .map(|(index, out_token)| {
                        SaleOutputOutToken {
                            remaining: out_token.remaining,
                            distributed: out_token.distributed,
                            treasury_unclaimed: out_token.treasury_unclaimed,
                            ..(sale.out_tokens[index].clone())
                        }
                    })
                    .collect(),
                in_token_remaining: expected.in_token_remaining,
                in_token_paid_unclaimed: expected.in_token_paid_unclaimed,
                in_token_paid: expected.in_token_paid,
                total_shares: expected.total_shares,
                subscription: expected.subscription,
                ..(sale.clone())
            }
        );
    }
}
