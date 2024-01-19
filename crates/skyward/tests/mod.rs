mod util;

use near_contract_standards::fungible_token::metadata::{FungibleTokenMetadata, FT_METADATA_SPEC};
use near_sdk::{env, json_types::U128, serde_json::json, Gas, NearToken};
use near_workspaces::{
    types::{KeyType, SecretKey},
    AccountId,
};
use skyward::{SaleInput, SaleInputOutToken, SaleOutput, SaleOutputOutToken, SubscriptionOutput};
use util::*;

const SKYWARD_WASM_BYTES: &[u8] = include_bytes!("../../../res/skyward_testing.wasm");
const FUNGIBLE_TOKEN_WASM_BYTES: &[u8] = include_bytes!("../../../common/fungible_token.wasm");
const W_NEAR_WASM_BYTES: &[u8] = include_bytes!("../../../common/w_near.wasm");
const PERMISSIONS_WASM_BYTES: &[u8] = include_bytes!("../../../res/permissions.wasm");

const TITLE: &str = "sale title";
const SKYWARD_ID: &str = "skyward.test.near";
const WRAP_NEAR_ID: &str = "wrap.test.near";
const SKYWARD_TOKEN_ID: &str = "token-skyward.test.near";
const SKYWARD_DAO_ID: &str = "skyward-dao.test.near";
const PERMISSIONS_CONTRACT_ID: &str = "kyc.test.near";

const TOKEN1_ID: &str = "token1.test.near";

const DAY: u32 = 24 * 60 * 60;
const WEEK: u32 = 7 * DAY;
const SKYWARD_TOKEN_DECIMALS: u8 = 18;
const SKYWARD_TOKEN_BASE: u128 = 10u128.pow(SKYWARD_TOKEN_DECIMALS as u32);
const SKYWARD_TOTAL_SUPPLY: u128 = 1_000_000 * SKYWARD_TOKEN_BASE;
const LISTING_FEE_NEAR: NearToken = NearToken::from_near(10);
const DEFAULT_TOTAL_SUPPLY: u128 = NearToken::from_near(1_000_000_000).as_yoctonear();

const BLOCK_DURATION: u64 = 1_000_000_000;

#[derive(Debug, PartialEq)]
pub struct PartialSale {
    pub out_tokens: Vec<PartialOutToken>,

    pub in_token_remaining: U128,
    pub in_token_paid_unclaimed: U128,
    pub in_token_paid: U128,

    pub total_shares: U128,

    pub subscription: Option<SubscriptionOutput>,
}

#[derive(Debug, PartialEq)]
pub struct PartialOutToken {
    pub remaining: U128,
    pub distributed: U128,
    pub treasury_unclaimed: U128,
}

#[tokio::test]
async fn test_init() -> anyhow::Result<()> {
    Env::init(0).await?;
    Ok(())
}

#[tokio::test]
async fn test_account_deposit() -> anyhow::Result<()> {
    let environment = Env::init(1).await?;
    let alice = environment.users.first().unwrap();

    assert_eq!(
        environment.balances_of(alice).await?,
        vec![(
            environment.w_near.id().clone(),
            NearToken::from_near(10).as_yoctonear()
        )]
    );

    Ok(())
}

#[tokio::test]
async fn test_wrap_extra_near() -> anyhow::Result<()> {
    let environment = Env::init(0).await?;

    assert_eq!(environment.get_treasury_balances().await?, vec![]);

    environment
        .worker
        .root_account()?
        .transfer_near(environment.skyward.id(), NearToken::from_near(9000))
        .await?
        .into_result()?;

    assert_eq!(
        environment
            .get_token_balance(environment.w_near.id(), environment.skyward.as_account())
            .await?,
        0
    );

    let initial_balance = environment.skyward.view_account().await?.balance;

    let res: bool = environment
        .skyward
        .call("wrap_extra_near")
        .max_gas()
        .transact()
        .await?
        .into_result()?
        .json()?;
    assert!(res);

    let near_spent = initial_balance.as_yoctonear()
        - environment
            .skyward
            .view_account()
            .await?
            .balance
            .as_yoctonear();
    assert!(near_spent > NearToken::from_near(9000).as_yoctonear());

    let w_near_balance = environment.get_treasury_balances().await?[0].1;
    assert!(w_near_balance > NearToken::from_near(9000).as_yoctonear());
    assert_eq!(
        environment
            .get_token_balance(environment.w_near.id(), environment.skyward.as_account())
            .await?,
        w_near_balance
    );

    assert!(environment
        .skyward
        .call("wrap_extra_near")
        .max_gas()
        .transact()
        .await?
        .into_result()
        .is_err());

    environment
        .worker
        .root_account()?
        .transfer_near(environment.skyward.id(), NearToken::from_millinear(10_100))
        .await?
        .into_result()?;

    let initial_balance = environment.skyward.view_account().await?.balance;

    let res: bool = environment
        .skyward
        .call("wrap_extra_near")
        .max_gas()
        .transact()
        .await?
        .into_result()?
        .json()?;
    assert!(res);

    let near_spent = initial_balance.as_yoctonear()
        - environment
            .skyward
            .view_account()
            .await?
            .balance
            .as_yoctonear();
    assert!(near_spent > NearToken::from_near(10).as_yoctonear());

    let w_near_balance_addition = environment.get_treasury_balances().await?[0].1 - w_near_balance;
    assert!(w_near_balance_addition > NearToken::from_near(10).as_yoctonear());
    assert_eq!(
        environment
            .get_token_balance(environment.w_near.id(), environment.skyward.as_account())
            .await?,
        w_near_balance + w_near_balance_addition
    );

    Ok(())
}

#[tokio::test]
async fn test_create_sale() -> anyhow::Result<()> {
    let environment = Env::init(1).await?;
    let alice = environment.users.first().unwrap();

    let token1 = environment.deploy_ft(alice.id(), TOKEN1_ID).await?;
    environment
        .register_and_deposit(alice, token1.id(), NearToken::from_near(10000))
        .await?;

    let start_offset = to_nano(WEEK) + BLOCK_DURATION * 15;
    let current_time = environment
        .worker
        .view_block()
        .await?
        .header()
        .timestamp_nanosec();
    let start_time = current_time + start_offset;
    let sale = environment
        .sale_create(
            alice,
            &[(
                token1.as_account(),
                NearToken::from_near(4000).as_yoctonear(),
            )],
            start_time,
        )
        .await?;

    let current_block = environment.worker.view_block().await?;
    assert_eq!(
        sale,
        SaleOutput {
            sale_id: 0,
            title: TITLE.to_string(),
            url: None,
            permissions_contract_id: None,
            owner_id: alice.id().parse()?,
            out_tokens: vec![SaleOutputOutToken {
                token_account_id: token1.id().parse()?,
                remaining: NearToken::from_near(4000).as_yoctonear().into(),
                distributed: 0.into(),
                treasury_unclaimed: 0.into(),
                referral_bpt: None
            }],
            in_token_account_id: environment.w_near.id().parse()?,
            in_token_remaining: U128(0),
            in_token_paid_unclaimed: U128(0),
            in_token_paid: U128(0),
            total_shares: U128(0),
            start_time: start_time.into(),
            duration: (BLOCK_DURATION * 60).into(),
            remaining_duration: (BLOCK_DURATION * 60).into(),
            subscription: None,
            current_time: current_block.timestamp().into(),
            current_block_height: current_block.height().into(),
            start_block_height: sale.start_block_height,
            end_block_height: None
        },
    );

    assert_eq!(
        environment.balances_of(alice).await?,
        vec![
            (
                environment.w_near.id().clone(),
                NearToken::from_near(10).as_yoctonear()
            ),
            (
                token1.id().clone(),
                NearToken::from_near(6000).as_yoctonear()
            ),
        ]
    );

    Ok(())
}

#[tokio::test]
async fn test_join_sale() -> anyhow::Result<()> {
    let environment = Env::init(2).await?;
    let alice = environment.users.first().unwrap();
    let bob = environment.users.get(1).unwrap();

    let token1 = environment.deploy_ft(alice.id(), TOKEN1_ID).await?;
    environment
        .register_and_deposit(alice, token1.id(), NearToken::from_near(10_000))
        .await?;

    let start_offset = BLOCK_DURATION * 15;
    let current_time = environment
        .worker
        .view_block()
        .await?
        .header()
        .timestamp_nanosec();
    let start_time = current_time + start_offset;
    let sale = environment
        .sale_create(
            alice,
            &[(
                token1.as_account(),
                NearToken::from_near(3_600).as_yoctonear(),
            )],
            start_time,
        )
        .await?;

    log_tx_result(
        "sale_deposit_in_token",
        bob.call(environment.skyward.id(), "sale_deposit_in_token")
            .args_json((
                sale.sale_id,
                U128(NearToken::from_near(4).as_yoctonear()),
                None::<AccountId>,
            ))
            .deposit(NearToken::from_millinear(10))
            .transact()
            .await?,
    )?;

    let bobs_sale = environment.get_sale(0, Some(bob.id().clone())).await?;
    assert_eq!(
        bobs_sale.in_token_remaining.0,
        NearToken::from_near(4).as_yoctonear()
    );
    assert_eq!(
        bobs_sale.total_shares.0,
        NearToken::from_near(4).as_yoctonear()
    );
    assert_eq!(
        bobs_sale.subscription,
        Some(SubscriptionOutput {
            claimed_out_balance: vec![0.into()],
            spent_in_balance: 0.into(),
            remaining_in_balance: NearToken::from_near(4).as_yoctonear().into(),
            unclaimed_out_balances: vec![U128(0)],
            shares: NearToken::from_near(4).as_yoctonear().into(),
            referral_id: None
        })
    );

    assert_eq!(
        environment.balances_of(bob).await?,
        vec![
            (
                environment.w_near.id().clone(),
                NearToken::from_near(6).as_yoctonear()
            ),
            (token1.id().clone(), 0),
        ]
    );

    environment.worker.fast_forward(500).await?;

    let bobs_sale = environment.get_sale(0, Some(bob.id().clone())).await?;

    environment.assert_sale_eq(
        &bobs_sale,
        PartialSale {
            out_tokens: vec![PartialOutToken {
                remaining: 0.into(),
                distributed: NearToken::from_near(3_600).as_yoctonear().into(),
                treasury_unclaimed: NearToken::from_near(36).as_yoctonear().into(),
            }],
            in_token_remaining: 0.into(),
            in_token_paid_unclaimed: NearToken::from_near(4).as_yoctonear().into(),
            in_token_paid: NearToken::from_near(4).as_yoctonear().into(),
            total_shares: NearToken::from_near(4).as_yoctonear().into(),
            subscription: Some(SubscriptionOutput {
                claimed_out_balance: vec![0.into()],
                spent_in_balance: NearToken::from_near(4).as_yoctonear().into(),
                remaining_in_balance: 0.into(),
                unclaimed_out_balances: vec![NearToken::from_near(3564).as_yoctonear().into()],
                shares: NearToken::from_near(4).as_yoctonear().into(),
                referral_id: None,
            }),
        },
    );

    assert_eq!(
        environment.get_treasury_balances().await?,
        vec![
            (environment.w_near.id().clone(), 0),
            (token1.id().clone(), 0),
        ]
    );

    log_tx_result(
        "sale_distribute_unclaimed_tokens",
        alice
            .call(environment.skyward.id(), "sale_distribute_unclaimed_tokens")
            .args_json((0,))
            .transact()
            .await?,
    )?;

    assert_eq!(
        environment.balances_of(alice).await?,
        vec![
            (
                environment.w_near.id().clone(),
                NearToken::from_millinear(13_960).as_yoctonear()
            ),
            (
                token1.id().clone(),
                NearToken::from_near(6400).as_yoctonear()
            ),
        ]
    );
    assert_eq!(
        environment.get_treasury_balances().await?,
        vec![
            (
                environment.w_near.id().clone(),
                NearToken::from_millinear(40).as_yoctonear()
            ),
            (token1.id().clone(), NearToken::from_near(36).as_yoctonear()),
        ]
    );
    assert_eq!(
        environment.balances_of(bob).await?,
        vec![
            (
                environment.w_near.id().clone(),
                NearToken::from_near(6).as_yoctonear()
            ),
            (token1.id().clone(), 0),
        ]
    );

    let bobs_sale = environment.get_sale(0, Some(bob.id().clone())).await?;

    environment.assert_sale_eq(
        &bobs_sale,
        PartialSale {
            out_tokens: vec![PartialOutToken {
                remaining: 0.into(),
                distributed: NearToken::from_near(3600).as_yoctonear().into(),
                treasury_unclaimed: 0.into(),
            }],
            in_token_remaining: 0.into(),
            in_token_paid_unclaimed: 0.into(),
            in_token_paid: NearToken::from_near(4).as_yoctonear().into(),
            total_shares: NearToken::from_near(4).as_yoctonear().into(),
            subscription: Some(SubscriptionOutput {
                claimed_out_balance: vec![0.into()],
                spent_in_balance: NearToken::from_near(4).as_yoctonear().into(),
                remaining_in_balance: 0.into(),
                unclaimed_out_balances: vec![NearToken::from_near(3564).as_yoctonear().into()],
                shares: NearToken::from_near(4).as_yoctonear().into(),
                referral_id: None,
            }),
        },
    );

    log_tx_result(
        "sale_claim_out_tokens",
        bob.call(environment.skyward.id(), "sale_claim_out_tokens")
            .args_json((0,))
            .transact()
            .await?,
    )?;

    let bobs_sale = environment.get_sale(0, Some(bob.id().clone())).await?;

    environment.assert_sale_eq(
        &bobs_sale,
        PartialSale {
            out_tokens: vec![PartialOutToken {
                remaining: 0.into(),
                distributed: NearToken::from_near(3600).as_yoctonear().into(),
                treasury_unclaimed: 0.into(),
            }],
            in_token_remaining: 0.into(),
            in_token_paid_unclaimed: 0.into(),
            in_token_paid: NearToken::from_near(4).as_yoctonear().into(),
            total_shares: 0.into(),
            subscription: None,
        },
    );

    assert_eq!(
        environment.get_treasury_balances().await?,
        vec![
            (
                environment.w_near.id().clone(),
                NearToken::from_millinear(40).as_yoctonear()
            ),
            (token1.id().clone(), NearToken::from_near(36).as_yoctonear()),
        ]
    );
    assert_eq!(
        environment.balances_of(bob).await?,
        vec![
            (
                environment.w_near.id().clone(),
                NearToken::from_near(6).as_yoctonear()
            ),
            (
                token1.id().clone(),
                NearToken::from_near(3564).as_yoctonear()
            ),
        ]
    );

    environment
        .storage_deposit(&token1, bob, None, Some(NearToken::from_millinear(50)))
        .await?;
    environment.withdraw_token(bob, token1.id(), None).await?;

    assert_eq!(
        environment.balances_of(bob).await?,
        vec![
            (
                environment.w_near.id().clone(),
                NearToken::from_near(6).as_yoctonear()
            ),
            (token1.id().clone(), 0),
        ]
    );
    assert_eq!(
        environment.ft_balance_of(bob, token1.id()).await?,
        NearToken::from_near(3564).as_yoctonear()
    );

    Ok(())
}

#[tokio::test]
async fn test_join_sale_with_referral() -> anyhow::Result<()> {
    let environment = Env::init(2).await?;
    let alice = environment.users.first().unwrap();
    let bob = environment.users.get(1).unwrap();

    let sale_amount = NearToken::from_near(10_000).as_yoctonear();
    let token1 = environment.deploy_ft(alice.id(), TOKEN1_ID).await?;
    environment
        .register_and_deposit(alice, token1.id(), NearToken::from_yoctonear(sale_amount))
        .await?;

    let start_offset = BLOCK_DURATION * 15;
    let current_time = environment
        .worker
        .view_block()
        .await?
        .header()
        .timestamp_nanosec();
    let start_time = current_time + start_offset;
    let sale = environment
        .sale_create_with_ref(alice, &[(token1.as_account(), sale_amount)], start_time)
        .await?;

    assert_eq!(
        environment.balances_of(alice).await?,
        vec![
            (
                environment.w_near.id().clone(),
                NearToken::from_near(10).as_yoctonear()
            ),
            (token1.id().clone(), 0),
        ]
    );

    log_tx_result(
        "sale_deposit_in_token",
        bob.call(environment.skyward.id(), "sale_deposit_in_token")
            .args_json((
                sale.sale_id,
                U128(NearToken::from_near(4).as_yoctonear()),
                Some(alice.id().clone()),
            ))
            .deposit(NearToken::from_millinear(10))
            .transact()
            .await?,
    )?;

    environment.worker.fast_forward(500).await?;

    let bobs_sale = environment.get_sale(0, Some(bob.id().clone())).await?;

    environment.assert_sale_eq(
        &bobs_sale,
        PartialSale {
            out_tokens: vec![PartialOutToken {
                remaining: 0.into(),
                distributed: sale_amount.into(),
                treasury_unclaimed: (sale_amount / 100).into(),
            }],
            in_token_remaining: 0.into(),
            in_token_paid_unclaimed: NearToken::from_near(4).as_yoctonear().into(),
            in_token_paid: NearToken::from_near(4).as_yoctonear().into(),
            total_shares: NearToken::from_near(4).as_yoctonear().into(),
            subscription: Some(SubscriptionOutput {
                claimed_out_balance: vec![0.into()],
                spent_in_balance: NearToken::from_near(4).as_yoctonear().into(),
                remaining_in_balance: 0.into(),
                unclaimed_out_balances: vec![(sale_amount * 99 / 100).into()],
                shares: NearToken::from_near(4).as_yoctonear().into(),
                referral_id: Some(alice.id().parse()?),
            }),
        },
    );

    assert_eq!(
        environment.get_treasury_balances().await?,
        vec![
            (environment.w_near.id().clone(), 0),
            (token1.id().clone(), 0)
        ]
    );

    log_tx_result(
        "sale_distribute_unclaimed_tokens",
        alice
            .call(environment.skyward.id(), "sale_distribute_unclaimed_tokens")
            .args_json((0,))
            .transact()
            .await?,
    )?;

    assert_eq!(
        environment.get_treasury_balances().await?,
        vec![
            (
                environment.w_near.id().clone(),
                NearToken::from_millinear(40).as_yoctonear()
            ),
            (token1.id().clone(), sale_amount / 100),
        ]
    );
    assert_eq!(
        environment.balances_of(alice).await?,
        vec![
            (
                environment.w_near.id().clone(),
                NearToken::from_millinear(13_960).as_yoctonear()
            ),
            (token1.id().clone(), 0),
        ]
    );
    assert_eq!(
        environment.balances_of(bob).await?,
        vec![
            (
                environment.w_near.id().clone(),
                NearToken::from_near(6).as_yoctonear()
            ),
            (token1.id().clone(), 0),
        ]
    );

    let bobs_sale = environment.get_sale(0, Some(bob.id().clone())).await?;

    environment.assert_sale_eq(
        &bobs_sale,
        PartialSale {
            out_tokens: vec![PartialOutToken {
                remaining: 0.into(),
                distributed: sale_amount.into(),
                treasury_unclaimed: 0.into(),
            }],
            in_token_remaining: 0.into(),
            in_token_paid_unclaimed: 0.into(),
            in_token_paid: NearToken::from_near(4).as_yoctonear().into(),
            total_shares: NearToken::from_near(4).as_yoctonear().into(),
            subscription: Some(SubscriptionOutput {
                claimed_out_balance: vec![0.into()],
                spent_in_balance: NearToken::from_near(4).as_yoctonear().into(),
                remaining_in_balance: 0.into(),
                unclaimed_out_balances: vec![(sale_amount * 99 / 100).into()],
                shares: NearToken::from_near(4).as_yoctonear().into(),
                referral_id: Some(alice.id().parse()?),
            }),
        },
    );

    log_tx_result(
        "sale_claim_out_tokens",
        bob.call(environment.skyward.id(), "sale_claim_out_tokens")
            .args_json((0,))
            .transact()
            .await?,
    )?;

    let bobs_sale = environment.get_sale(0, Some(bob.id().clone())).await?;

    environment.assert_sale_eq(
        &bobs_sale,
        PartialSale {
            out_tokens: vec![PartialOutToken {
                remaining: 0.into(),
                distributed: sale_amount.into(),
                treasury_unclaimed: 0.into(),
            }],
            in_token_remaining: 0.into(),
            in_token_paid_unclaimed: 0.into(),
            in_token_paid: NearToken::from_near(4).as_yoctonear().into(),
            total_shares: 0.into(),
            subscription: None,
        },
    );

    let out_amount = sale_amount * 99 / 100;
    let ref_amount = out_amount / 100 / 2;
    assert_eq!(
        environment.balances_of(alice).await?,
        vec![
            (
                environment.w_near.id().clone(),
                NearToken::from_millinear(13_960).as_yoctonear()
            ),
            (token1.id().clone(), ref_amount),
        ]
    );
    assert_eq!(
        environment.balances_of(bob).await?,
        vec![
            (
                environment.w_near.id().clone(),
                NearToken::from_near(6).as_yoctonear()
            ),
            (token1.id().clone(), out_amount - ref_amount),
        ]
    );

    environment.claim_treasury(alice).await?;

    assert_eq!(
        environment
            .ft_balance_of(&environment.skyward_dao, environment.w_near.id())
            .await?,
        NearToken::from_millinear(40).as_yoctonear()
    );
    assert_eq!(
        environment
            .ft_balance_of(&environment.skyward_dao, token1.id())
            .await?,
        sale_amount / 100
    );

    Ok(())
}

// #[test]
// fn test_join_sale_with_referral_and_alice() {
//     let e = Env::init(2);
//     let alice = e.users.get(0).unwrap();
//     let bob = e.users.get(1).unwrap();

//     let sale_amount = 10000 * SKYWARD_TOKEN_BASE;
//     e.register_and_deposit(&e.skyward_dao, &e.skyward_token, sale_amount * 2);

//     e.register_skyward_token(alice);
//     assert_eq!(e.skyward_circulating_supply(), SKYWARD_TOTAL_SUPPLY);

//     let sale = e.sale_create_with_ref(&e.skyward_dao, &[(&e.skyward_token, sale_amount)]);
//     assert_eq!(e.skyward_circulating_supply(), SKYWARD_TOTAL_SUPPLY);

//     assert_eq!(
//         e.balances_of(&e.skyward_dao),
//         vec![
//             (e.skyward_token.account_id.clone(), sale_amount),
//             (e.w_near.account_id.clone(), to_yocto("0")),
//         ]
//     );

//     bob.function_call(
//         e.skyward.contract.sale_deposit_in_token(
//             sale.sale_id,
//             to_yocto("4").into(),
//             Some(alice.valid_account_id()),
//         ),
//         BASE_GAS,
//         to_yocto("0.01"),
//     )
//     .assert_success();

//     alice
//         .function_call(
//             e.skyward
//                 .contract
//                 .sale_deposit_in_token(sale.sale_id, to_yocto("1").into(), None),
//             BASE_GAS,
//             to_yocto("0.01"),
//         )
//         .assert_success();

//     let bobs_sale = e.get_sale(0, Some(bob.valid_account_id()));
//     e.assert_sale_eq(
//         &bobs_sale,
//         PartialSale {
//             out_tokens: vec![PartialOutToken {
//                 remaining: sale_amount.into(),
//                 distributed: 0.into(),
//                 treasury_unclaimed: None,
//             }],
//             in_token_remaining: to_yocto("5").into(),
//             in_token_paid_unclaimed: to_yocto("0").into(),
//             in_token_paid: to_yocto("0").into(),
//             total_shares: to_yocto("5").into(),
//             subscription: Some(SubscriptionOutput {
//                 claimed_out_balance: vec![to_yocto("0").into()],
//                 spent_in_balance: to_yocto("0").into(),
//                 remaining_in_balance: to_yocto("4").into(),
//                 unclaimed_out_balances: vec![U128(0)],
//                 shares: to_yocto("4").into(),
//                 referral_id: Some(alice.account_id.clone()),
//             }),
//         },
//     );

//     assert_eq!(
//         e.get_sale(0, Some(alice.valid_account_id())).subscription,
//         Some(SubscriptionOutput {
//             claimed_out_balance: vec![to_yocto("0").into()],
//             spent_in_balance: to_yocto("0").into(),
//             remaining_in_balance: to_yocto("1").into(),
//             unclaimed_out_balances: vec![U128(0)],
//             shares: to_yocto("1").into(),
//             referral_id: None
//         }),
//     );

//     assert_eq!(
//         e.balances_of(alice),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("9")),
//             (e.skyward_token.account_id.clone(), 0),
//         ]
//     );
//     assert_eq!(
//         e.balances_of(bob),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("6")),
//             (e.skyward_token.account_id.clone(), 0),
//         ]
//     );

//     e.near.borrow_runtime_mut().cur_block.block_timestamp = sale.start_time.0 + sale.duration.0 / 2;

//     let bobs_sale = e.get_sale(0, Some(bob.valid_account_id()));
//     e.assert_sale_eq(
//         &bobs_sale,
//         PartialSale {
//             out_tokens: vec![PartialOutToken {
//                 remaining: (sale_amount / 2).into(),
//                 distributed: (sale_amount / 2).into(),
//                 treasury_unclaimed: None,
//             }],
//             in_token_remaining: to_yocto("2.5").into(),
//             in_token_paid_unclaimed: to_yocto("2.5").into(),
//             in_token_paid: to_yocto("2.5").into(),
//             total_shares: to_yocto("5").into(),
//             subscription: Some(SubscriptionOutput {
//                 claimed_out_balance: vec![to_yocto("0").into()],
//                 spent_in_balance: to_yocto("2").into(),
//                 remaining_in_balance: to_yocto("2").into(),
//                 unclaimed_out_balances: vec![(sale_amount / 5 * 4 / 2).into()],
//                 shares: to_yocto("4").into(),
//                 referral_id: Some(alice.account_id.clone()),
//             }),
//         },
//     );

//     assert_eq!(
//         e.get_sale(0, Some(alice.valid_account_id())).subscription,
//         Some(SubscriptionOutput {
//             claimed_out_balance: vec![to_yocto("0").into()],
//             spent_in_balance: to_yocto("0.5").into(),
//             remaining_in_balance: to_yocto("0.5").into(),
//             unclaimed_out_balances: vec![(sale_amount / 5 * 1 / 2).into()],
//             shares: to_yocto("1").into(),
//             referral_id: None
//         }),
//     );

//     e.near.borrow_runtime_mut().cur_block.block_timestamp = sale.start_time.0 + sale.duration.0;

//     let bobs_sale = e.get_sale(0, Some(bob.valid_account_id()));
//     e.assert_sale_eq(
//         &bobs_sale,
//         PartialSale {
//             out_tokens: vec![PartialOutToken {
//                 remaining: 0.into(),
//                 distributed: sale_amount.into(),
//                 treasury_unclaimed: None,
//             }],
//             in_token_remaining: to_yocto("0").into(),
//             in_token_paid_unclaimed: to_yocto("5").into(),
//             in_token_paid: to_yocto("5").into(),
//             total_shares: to_yocto("5").into(),
//             subscription: Some(SubscriptionOutput {
//                 claimed_out_balance: vec![to_yocto("0").into()],
//                 spent_in_balance: to_yocto("4").into(),
//                 remaining_in_balance: 0.into(),
//                 unclaimed_out_balances: vec![(sale_amount / 5 * 4).into()],
//                 shares: to_yocto("4").into(),
//                 referral_id: Some(alice.account_id.clone()),
//             }),
//         },
//     );

//     assert_eq!(
//         e.get_sale(0, Some(alice.valid_account_id())).subscription,
//         Some(SubscriptionOutput {
//             claimed_out_balance: vec![to_yocto("0").into()],
//             spent_in_balance: to_yocto("1").into(),
//             remaining_in_balance: 0.into(),
//             unclaimed_out_balances: vec![(sale_amount / 5).into()],
//             shares: to_yocto("1").into(),
//             referral_id: None
//         }),
//     );

//     assert_eq!(
//         e.get_treasury_balances(),
//         vec![(e.w_near.account_id.clone(), 0),]
//     );
//     assert_eq!(e.skyward_circulating_supply(), SKYWARD_TOTAL_SUPPLY);

//     alice
//         .function_call(
//             e.skyward.contract.sale_distribute_unclaimed_tokens(0),
//             BASE_GAS,
//             0,
//         )
//         .assert_success();

//     assert_eq!(
//         e.balances_of(&e.skyward_dao),
//         vec![
//             (e.skyward_token.account_id.clone(), sale_amount),
//             (e.w_near.account_id.clone(), to_yocto("4.95")),
//         ]
//     );

//     assert_eq!(
//         e.balances_of(alice),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("9")),
//             (e.skyward_token.account_id.clone(), 0),
//         ]
//     );
//     assert_eq!(
//         e.get_treasury_balances(),
//         vec![(e.w_near.account_id.clone(), to_yocto("0.05")),]
//     );
//     assert_eq!(
//         e.balances_of(bob),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("6")),
//             (e.skyward_token.account_id.clone(), 0),
//         ]
//     );

//     let bobs_sale = e.get_sale(0, Some(bob.valid_account_id()));
//     e.assert_sale_eq(
//         &bobs_sale,
//         PartialSale {
//             out_tokens: vec![PartialOutToken {
//                 remaining: 0.into(),
//                 distributed: sale_amount.into(),
//                 treasury_unclaimed: None,
//             }],
//             in_token_remaining: to_yocto("0").into(),
//             in_token_paid_unclaimed: to_yocto("0").into(),
//             in_token_paid: to_yocto("5").into(),
//             total_shares: to_yocto("5").into(),
//             subscription: Some(SubscriptionOutput {
//                 claimed_out_balance: vec![to_yocto("0").into()],
//                 spent_in_balance: to_yocto("4").into(),
//                 remaining_in_balance: to_yocto("0").into(),
//                 unclaimed_out_balances: vec![(sale_amount / 5 * 4).into()],
//                 shares: to_yocto("4").into(),
//                 referral_id: Some(alice.account_id.clone()),
//             }),
//         },
//     );

//     assert_eq!(
//         e.get_sale(0, Some(alice.valid_account_id())).subscription,
//         Some(SubscriptionOutput {
//             claimed_out_balance: vec![to_yocto("0").into()],
//             spent_in_balance: to_yocto("1").into(),
//             remaining_in_balance: 0.into(),
//             unclaimed_out_balances: vec![(sale_amount / 5).into()],
//             shares: to_yocto("1").into(),
//             referral_id: None
//         }),
//     );

//     assert_eq!(e.skyward_circulating_supply(), SKYWARD_TOTAL_SUPPLY);

//     bob.function_call(e.skyward.contract.sale_claim_out_tokens(0), BASE_GAS, 0)
//         .assert_success();

//     let bobs_sale = e.get_sale(0, Some(bob.valid_account_id()));
//     e.assert_sale_eq(
//         &bobs_sale,
//         PartialSale {
//             out_tokens: vec![PartialOutToken {
//                 remaining: 0.into(),
//                 distributed: sale_amount.into(),
//                 treasury_unclaimed: None,
//             }],
//             in_token_remaining: to_yocto("0").into(),
//             in_token_paid_unclaimed: to_yocto("0").into(),
//             in_token_paid: to_yocto("5").into(),
//             total_shares: to_yocto("1").into(),
//             subscription: None,
//         },
//     );

//     assert_eq!(
//         e.get_sale(0, Some(alice.valid_account_id())).subscription,
//         Some(SubscriptionOutput {
//             claimed_out_balance: vec![to_yocto("0").into()],
//             spent_in_balance: to_yocto("1").into(),
//             remaining_in_balance: 0.into(),
//             unclaimed_out_balances: vec![(sale_amount / 5).into()],
//             shares: to_yocto("1").into(),
//             referral_id: None
//         }),
//     );

//     assert_eq!(e.skyward_circulating_supply(), SKYWARD_TOTAL_SUPPLY);
//     assert_eq!(
//         e.get_treasury_balances(),
//         vec![(e.w_near.account_id.clone(), to_yocto("0.05")),]
//     );
//     assert_eq!(
//         e.balances_of(alice),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("9")),
//             (
//                 e.skyward_token.account_id.clone(),
//                 sale_amount / 5 * 4 / 200
//             ),
//         ]
//     );
//     assert_eq!(
//         e.balances_of(bob),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("6")),
//             (
//                 e.skyward_token.account_id.clone(),
//                 sale_amount / 5 * 4 / 200 * 199
//             ),
//         ]
//     );

//     alice
//         .function_call(e.skyward.contract.sale_claim_out_tokens(0), BASE_GAS, 0)
//         .assert_success();

//     let alice_sale = e.get_sale(0, Some(alice.valid_account_id()));
//     e.assert_sale_eq(
//         &alice_sale,
//         PartialSale {
//             out_tokens: vec![PartialOutToken {
//                 remaining: 0.into(),
//                 distributed: sale_amount.into(),
//                 treasury_unclaimed: None,
//             }],
//             in_token_remaining: to_yocto("0").into(),
//             in_token_paid_unclaimed: to_yocto("0").into(),
//             in_token_paid: to_yocto("5").into(),
//             total_shares: to_yocto("0").into(),
//             subscription: None,
//         },
//     );

//     e.assert_sale_eq(
//         &alice_sale,
//         PartialSale {
//             out_tokens: vec![PartialOutToken {
//                 remaining: 0.into(),
//                 distributed: sale_amount.into(),
//                 treasury_unclaimed: None,
//             }],
//             in_token_remaining: to_yocto("0").into(),
//             in_token_paid_unclaimed: to_yocto("0").into(),
//             in_token_paid: to_yocto("5").into(),
//             total_shares: to_yocto("0").into(),
//             subscription: None,
//         },
//     );

//     assert_eq!(e.skyward_circulating_supply(), SKYWARD_TOTAL_SUPPLY);

//     assert_eq!(
//         e.balances_of(alice),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("9")),
//             (
//                 e.skyward_token.account_id.clone(),
//                 sale_amount / 5 * 99 / 100 + sale_amount / 5 * 4 / 200
//             ),
//         ]
//     );
//     assert_eq!(
//         e.get_treasury_balances(),
//         vec![(e.w_near.account_id.clone(), to_yocto("0.05")),]
//     );
// }

// #[test]
// fn test_join_sale_and_leave() {
//     let e = Env::init(2);
//     let alice = e.users.get(0).unwrap();
//     let bob = e.users.get(1).unwrap();

//     let sale_amount = 10000 * SKYWARD_TOKEN_BASE;
//     e.register_and_deposit(&e.skyward_dao, &e.skyward_token, sale_amount * 2);

//     e.register_skyward_token(alice);
//     assert_eq!(e.skyward_circulating_supply(), SKYWARD_TOTAL_SUPPLY);

//     let sale = e.sale_create_with_ref(&e.skyward_dao, &[(&e.skyward_token, sale_amount)]);
//     assert_eq!(e.skyward_circulating_supply(), SKYWARD_TOTAL_SUPPLY);

//     assert_eq!(
//         e.balances_of(&e.skyward_dao),
//         vec![
//             (e.skyward_token.account_id.clone(), sale_amount),
//             (e.w_near.account_id.clone(), to_yocto("0")),
//         ]
//     );

//     bob.function_call(
//         e.skyward.contract.sale_deposit_in_token(
//             sale.sale_id,
//             to_yocto("4").into(),
//             Some(alice.valid_account_id()),
//         ),
//         BASE_GAS,
//         to_yocto("0.01"),
//     )
//     .assert_success();

//     alice
//         .function_call(
//             e.skyward
//                 .contract
//                 .sale_deposit_in_token(sale.sale_id, to_yocto("1").into(), None),
//             BASE_GAS,
//             to_yocto("0.01"),
//         )
//         .assert_success();

//     let bobs_sale = e.get_sale(0, Some(bob.valid_account_id()));
//     e.assert_sale_eq(
//         &bobs_sale,
//         PartialSale {
//             out_tokens: vec![PartialOutToken {
//                 remaining: sale_amount.into(),
//                 distributed: 0.into(),
//                 treasury_unclaimed: None,
//             }],
//             in_token_remaining: to_yocto("5").into(),
//             in_token_paid_unclaimed: to_yocto("0").into(),
//             in_token_paid: to_yocto("0").into(),
//             total_shares: to_yocto("5").into(),
//             subscription: Some(SubscriptionOutput {
//                 claimed_out_balance: vec![to_yocto("0").into()],
//                 spent_in_balance: to_yocto("0").into(),
//                 remaining_in_balance: to_yocto("4").into(),
//                 unclaimed_out_balances: vec![U128(0)],
//                 shares: to_yocto("4").into(),
//                 referral_id: Some(alice.account_id.clone()),
//             }),
//         },
//     );

//     assert_eq!(
//         e.get_sale(0, Some(alice.valid_account_id())).subscription,
//         Some(SubscriptionOutput {
//             claimed_out_balance: vec![to_yocto("0").into()],
//             spent_in_balance: to_yocto("0").into(),
//             remaining_in_balance: to_yocto("1").into(),
//             unclaimed_out_balances: vec![U128(0)],
//             shares: to_yocto("1").into(),
//             referral_id: None
//         }),
//     );

//     assert_eq!(
//         e.balances_of(alice),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("9")),
//             (e.skyward_token.account_id.clone(), 0),
//         ]
//     );
//     assert_eq!(
//         e.balances_of(bob),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("6")),
//             (e.skyward_token.account_id.clone(), 0),
//         ]
//     );

//     {
//         let mut runtime = e.near.borrow_runtime_mut();
//         runtime.cur_block.block_timestamp = sale.start_time.0 + sale.duration.0 / 2;
//     }

//     let bobs_sale = e.get_sale(0, Some(bob.valid_account_id()));
//     e.assert_sale_eq(
//         &bobs_sale,
//         PartialSale {
//             out_tokens: vec![PartialOutToken {
//                 remaining: (sale_amount / 2).into(),
//                 distributed: (sale_amount / 2).into(),
//                 treasury_unclaimed: None,
//             }],
//             in_token_remaining: to_yocto("2.5").into(),
//             in_token_paid_unclaimed: to_yocto("2.5").into(),
//             in_token_paid: to_yocto("2.5").into(),
//             total_shares: to_yocto("5").into(),
//             subscription: Some(SubscriptionOutput {
//                 claimed_out_balance: vec![to_yocto("0").into()],
//                 spent_in_balance: to_yocto("2").into(),
//                 remaining_in_balance: to_yocto("2").into(),
//                 unclaimed_out_balances: vec![(sale_amount / 5 * 4 / 2).into()],
//                 shares: to_yocto("4").into(),
//                 referral_id: Some(alice.account_id.clone()),
//             }),
//         },
//     );

//     assert_eq!(
//         e.get_sale(0, Some(alice.valid_account_id())).subscription,
//         Some(SubscriptionOutput {
//             claimed_out_balance: vec![to_yocto("0").into()],
//             spent_in_balance: to_yocto("0.5").into(),
//             remaining_in_balance: to_yocto("0.5").into(),
//             unclaimed_out_balances: vec![(sale_amount / 5 * 1 / 2).into()],
//             shares: to_yocto("1").into(),
//             referral_id: None
//         }),
//     );

//     // Alice leaves sale
//     alice
//         .function_call(
//             e.skyward.contract.sale_withdraw_in_token(0, None),
//             BASE_GAS,
//             1,
//         )
//         .assert_success();

//     assert_eq!(
//         e.balances_of(alice),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("9.5")),
//             (
//                 e.skyward_token.account_id.clone(),
//                 sale_amount / 5 * 1 / 2 * 99 / 100
//             ),
//         ]
//     );

//     let alice_sale = e.get_sale(0, Some(alice.valid_account_id()));
//     assert_eq!(alice_sale.in_token_remaining.0, to_yocto("2"));
//     assert_eq!(alice_sale.total_shares.0, to_yocto("4"));
//     assert_eq!(alice_sale.subscription, None);

//     e.near.borrow_runtime_mut().cur_block.block_timestamp = sale.start_time.0 + sale.duration.0;

//     let bobs_sale = e.get_sale(0, Some(bob.valid_account_id()));
//     e.assert_sale_eq(
//         &bobs_sale,
//         PartialSale {
//             out_tokens: vec![PartialOutToken {
//                 remaining: 0.into(),
//                 distributed: sale_amount.into(),
//                 treasury_unclaimed: None,
//             }],
//             in_token_remaining: to_yocto("0").into(),
//             in_token_paid_unclaimed: to_yocto("2").into(),
//             in_token_paid: to_yocto("4.5").into(),
//             total_shares: to_yocto("4").into(),
//             subscription: Some(SubscriptionOutput {
//                 claimed_out_balance: vec![to_yocto("0").into()],
//                 spent_in_balance: to_yocto("4").into(),
//                 remaining_in_balance: 0.into(),
//                 unclaimed_out_balances: vec![(sale_amount * 9 / 10).into()],
//                 shares: to_yocto("4").into(),
//                 referral_id: Some(alice.account_id.clone()),
//             }),
//         },
//     );

//     assert_eq!(
//         e.get_treasury_balances(),
//         vec![(e.w_near.account_id.clone(), to_yocto("0.025"))]
//     );
//     assert_eq!(e.skyward_circulating_supply(), SKYWARD_TOTAL_SUPPLY);

//     alice
//         .function_call(
//             e.skyward.contract.sale_distribute_unclaimed_tokens(0),
//             BASE_GAS,
//             0,
//         )
//         .assert_success();

//     assert_eq!(
//         e.balances_of(&e.skyward_dao),
//         vec![
//             (
//                 e.skyward_token.account_id.clone(),
//                 sale_amount + sale_amount / 5 * 1 / 2 / 100
//             ),
//             (e.w_near.account_id.clone(), to_yocto("4.455")),
//         ]
//     );

//     assert_eq!(
//         e.balances_of(alice),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("9.5")),
//             (
//                 e.skyward_token.account_id.clone(),
//                 sale_amount / 5 * 1 / 2 * 99 / 100
//             ),
//         ]
//     );
//     assert_eq!(
//         e.get_treasury_balances(),
//         vec![(e.w_near.account_id.clone(), to_yocto("0.045")),]
//     );
//     assert_eq!(e.skyward_circulating_supply(), SKYWARD_TOTAL_SUPPLY);
//     assert_eq!(
//         e.balances_of(bob),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("6")),
//             (e.skyward_token.account_id.clone(), 0),
//         ]
//     );

//     let bobs_sale = e.get_sale(0, Some(bob.valid_account_id()));
//     e.assert_sale_eq(
//         &bobs_sale,
//         PartialSale {
//             out_tokens: vec![PartialOutToken {
//                 remaining: 0.into(),
//                 distributed: sale_amount.into(),
//                 treasury_unclaimed: None,
//             }],
//             in_token_remaining: to_yocto("0").into(),
//             in_token_paid_unclaimed: to_yocto("0").into(),
//             in_token_paid: to_yocto("4.5").into(),
//             total_shares: to_yocto("4").into(),
//             subscription: Some(SubscriptionOutput {
//                 claimed_out_balance: vec![to_yocto("0").into()],
//                 spent_in_balance: to_yocto("4").into(),
//                 remaining_in_balance: to_yocto("0").into(),
//                 unclaimed_out_balances: vec![(sale_amount * 9 / 10).into()],
//                 shares: to_yocto("4").into(),
//                 referral_id: Some(alice.account_id.clone()),
//             }),
//         },
//     );

//     bob.function_call(e.skyward.contract.sale_claim_out_tokens(0), BASE_GAS, 0)
//         .assert_success();

//     let bobs_sale = e.get_sale(0, Some(bob.valid_account_id()));
//     e.assert_sale_eq(
//         &bobs_sale,
//         PartialSale {
//             out_tokens: vec![PartialOutToken {
//                 remaining: 0.into(),
//                 distributed: sale_amount.into(),
//                 treasury_unclaimed: None,
//             }],
//             in_token_remaining: to_yocto("0").into(),
//             in_token_paid_unclaimed: to_yocto("0").into(),
//             in_token_paid: to_yocto("4.5").into(),
//             total_shares: to_yocto("0").into(),
//             subscription: None,
//         },
//     );

//     assert_eq!(
//         e.balances_of(&e.skyward_dao),
//         vec![
//             (
//                 e.skyward_token.account_id.clone(),
//                 sale_amount + sale_amount / 5 / 2 / 100
//             ),
//             (e.w_near.account_id.clone(), to_yocto("4.455")),
//         ]
//     );

//     assert_eq!(
//         e.get_treasury_balances(),
//         vec![(e.w_near.account_id.clone(), to_yocto("0.045")),]
//     );
//     assert_eq!(
//         e.balances_of(alice),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("9.5")),
//             (
//                 e.skyward_token.account_id.clone(),
//                 sale_amount / 5 * 1 / 2 * 99 / 100 + sale_amount * 9 / 10 / 200
//             ),
//         ]
//     );
//     assert_eq!(
//         e.balances_of(bob),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("6")),
//             (
//                 e.skyward_token.account_id.clone(),
//                 sale_amount * 9 / 10 / 200 * 199
//             ),
//         ]
//     );
// }

// #[test]
// fn test_join_sale_and_withdraw_exact() {
//     let e = Env::init(1);
//     let alice = e.users.get(0).unwrap();

//     let sale_amount = 10000 * SKYWARD_TOKEN_BASE;
//     e.register_and_deposit(&e.skyward_dao, &e.skyward_token, sale_amount * 2);

//     e.register_skyward_token(alice);
//     assert_eq!(e.skyward_circulating_supply(), SKYWARD_TOTAL_SUPPLY);

//     let sale = e.sale_create(&e.skyward_dao, &[(&e.skyward_token, sale_amount)]);
//     assert_eq!(e.skyward_circulating_supply(), SKYWARD_TOTAL_SUPPLY);

//     assert_eq!(
//         e.balances_of(&e.skyward_dao),
//         vec![
//             (e.skyward_token.account_id.clone(), sale_amount),
//             (e.w_near.account_id.clone(), to_yocto("0")),
//         ]
//     );

//     alice
//         .function_call(
//             e.skyward
//                 .contract
//                 .sale_deposit_in_token(sale.sale_id, to_yocto("4").into(), None),
//             BASE_GAS,
//             to_yocto("0.01"),
//         )
//         .assert_success();

//     e.near.borrow_runtime_mut().cur_block.block_timestamp = sale.start_time.0 + sale.duration.0 / 3;

//     assert_eq!(
//         e.balances_of(alice),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("6")),
//             (e.skyward_token.account_id.clone(), 0),
//         ]
//     );
//     alice
//         .function_call(
//             e.skyward
//                 .contract
//                 .sale_withdraw_in_token_exact(sale.sale_id, to_yocto("2").into()),
//             BASE_GAS,
//             1,
//         )
//         .assert_success();

//     assert_eq!(
//         e.balances_of(alice)[0],
//         (e.w_near.account_id.clone(), to_yocto("8")),
//     );
// }

// #[test]
// fn test_skyward_sale_alice_joins_in_the_middle() {
//     let e = Env::init_with_schedule(2, vec![]);
//     let alice = e.users.get(0).unwrap();
//     let bob = e.users.get(1).unwrap();

//     let sale_amount = 10000 * SKYWARD_TOKEN_BASE;
//     e.skyward_dao
//         .call(
//             e.skyward_token.account_id.clone(),
//             "ft_transfer",
//             &json!({
//                 "receiver_id": SKYWARD_ID,
//                 "amount": U128::from(sale_amount),
//             })
//             .to_string()
//             .into_bytes(),
//             BASE_GAS,
//             1,
//         )
//         .assert_success();
//     assert_eq!(
//         e.get_token_balance(&e.skyward_token, &e.skyward.user_account),
//         sale_amount
//     );

//     let sale = e.sale_create_with_ref(&e.skyward.user_account, &[(&e.skyward_token, sale_amount)]);

//     assert_eq!(e.skyward_circulating_supply(), 0);

//     bob.function_call(
//         e.skyward
//             .contract
//             .sale_deposit_in_token(sale.sale_id, to_yocto("4").into(), None),
//         BASE_GAS,
//         to_yocto("0.01"),
//     )
//     .assert_success();

//     let bobs_sale = e.get_sale(0, Some(bob.valid_account_id()));
//     e.assert_sale_eq(
//         &bobs_sale,
//         PartialSale {
//             out_tokens: vec![PartialOutToken {
//                 remaining: sale_amount.into(),
//                 distributed: 0.into(),
//                 treasury_unclaimed: None,
//             }],
//             in_token_remaining: to_yocto("4").into(),
//             in_token_paid_unclaimed: 0.into(),
//             in_token_paid: to_yocto("0").into(),
//             total_shares: to_yocto("4").into(),
//             subscription: Some(SubscriptionOutput {
//                 claimed_out_balance: vec![to_yocto("0").into()],
//                 spent_in_balance: to_yocto("0").into(),
//                 remaining_in_balance: to_yocto("4").into(),
//                 unclaimed_out_balances: vec![U128(0)],
//                 shares: to_yocto("4").into(),
//                 referral_id: None,
//             }),
//         },
//     );

//     assert_eq!(
//         e.balances_of(bob),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("6")),
//             (e.skyward_token.account_id.clone(), 0),
//         ]
//     );

//     e.near.borrow_runtime_mut().cur_block.block_timestamp = sale.start_time.0 + sale.duration.0 / 4;

//     let bobs_sale = e.get_sale(0, Some(bob.valid_account_id()));
//     e.assert_sale_eq(
//         &bobs_sale,
//         PartialSale {
//             out_tokens: vec![PartialOutToken {
//                 remaining: (sale_amount / 4 * 3).into(),
//                 distributed: (sale_amount / 4).into(),
//                 treasury_unclaimed: None,
//             }],
//             in_token_remaining: to_yocto("3").into(),
//             in_token_paid_unclaimed: to_yocto("1").into(),
//             in_token_paid: to_yocto("1").into(),
//             total_shares: to_yocto("4").into(),
//             subscription: Some(SubscriptionOutput {
//                 claimed_out_balance: vec![0.into()],
//                 spent_in_balance: to_yocto("1").into(),
//                 remaining_in_balance: to_yocto("3").into(),
//                 unclaimed_out_balances: vec![(sale_amount / 4).into()],
//                 shares: to_yocto("4").into(),
//                 referral_id: None,
//             }),
//         },
//     );

//     assert_eq!(e.skyward_circulating_supply(), sale_amount / 4);

//     bob.function_call(e.skyward.contract.sale_claim_out_tokens(0), BASE_GAS, 0)
//         .assert_success();

//     assert_eq!(
//         e.skyward_circulating_supply(),
//         sale_amount / 4 - sale_amount / 4 / 100
//     );

//     let bobs_sale = e.get_sale(0, Some(bob.valid_account_id()));
//     e.assert_sale_eq(
//         &bobs_sale,
//         PartialSale {
//             out_tokens: vec![PartialOutToken {
//                 remaining: (sale_amount / 4 * 3).into(),
//                 distributed: (sale_amount / 4).into(),
//                 treasury_unclaimed: None,
//             }],
//             in_token_remaining: to_yocto("3").into(),
//             in_token_paid_unclaimed: to_yocto("0").into(),
//             in_token_paid: to_yocto("1").into(),
//             total_shares: to_yocto("4").into(),
//             subscription: Some(SubscriptionOutput {
//                 claimed_out_balance: vec![(sale_amount / 4 * 99 / 100).into()],
//                 spent_in_balance: to_yocto("1").into(),
//                 remaining_in_balance: to_yocto("3").into(),
//                 unclaimed_out_balances: vec![0.into()],
//                 shares: to_yocto("4").into(),
//                 referral_id: None,
//             }),
//         },
//     );

//     assert_eq!(
//         e.get_treasury_balances(),
//         vec![(e.w_near.account_id.clone(), to_yocto("1"))]
//     );

//     e.near.borrow_runtime_mut().cur_block.block_timestamp = sale.start_time.0 + sale.duration.0 / 2;

//     let bobs_sale = e.get_sale(0, Some(bob.valid_account_id()));
//     e.assert_sale_eq(
//         &bobs_sale,
//         PartialSale {
//             out_tokens: vec![PartialOutToken {
//                 remaining: (sale_amount / 2).into(),
//                 distributed: (sale_amount / 2).into(),
//                 treasury_unclaimed: None,
//             }],
//             in_token_remaining: to_yocto("2").into(),
//             in_token_paid_unclaimed: to_yocto("1").into(),
//             in_token_paid: to_yocto("2").into(),
//             total_shares: to_yocto("4").into(),
//             subscription: Some(SubscriptionOutput {
//                 claimed_out_balance: vec![(sale_amount / 4 * 99 / 100).into()],
//                 spent_in_balance: to_yocto("2").into(),
//                 remaining_in_balance: to_yocto("2").into(),
//                 unclaimed_out_balances: vec![(sale_amount / 4).into()],
//                 shares: to_yocto("4").into(),
//                 referral_id: None,
//             }),
//         },
//     );

//     assert_eq!(
//         e.skyward_circulating_supply(),
//         sale_amount / 2 - sale_amount / 4 / 100
//     );

//     alice
//         .function_call(
//             e.skyward
//                 .contract
//                 .sale_deposit_in_token(sale.sale_id, to_yocto("3").into(), None),
//             BASE_GAS,
//             to_yocto("0.01"),
//         )
//         .assert_success();

//     let alice_sale = e.get_sale(0, Some(alice.valid_account_id()));
//     e.assert_sale_eq(
//         &alice_sale,
//         PartialSale {
//             out_tokens: vec![PartialOutToken {
//                 remaining: (sale_amount / 2).into(),
//                 distributed: (sale_amount / 2).into(),
//                 treasury_unclaimed: None,
//             }],
//             in_token_remaining: to_yocto("5").into(),
//             in_token_paid_unclaimed: to_yocto("0").into(),
//             in_token_paid: to_yocto("2").into(),
//             total_shares: to_yocto("10").into(),
//             subscription: Some(SubscriptionOutput {
//                 claimed_out_balance: vec![to_yocto("0").into()],
//                 spent_in_balance: to_yocto("0").into(),
//                 remaining_in_balance: to_yocto("3").into(),
//                 unclaimed_out_balances: vec![0.into()],
//                 shares: to_yocto("6").into(),
//                 referral_id: None,
//             }),
//         },
//     );

//     assert_eq!(
//         e.balances_of(alice),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("7")),
//             (e.skyward_token.account_id.clone(), 0),
//         ]
//     );

//     alice
//         .function_call(
//             e.skyward.contract.sale_distribute_unclaimed_tokens(0),
//             BASE_GAS,
//             0,
//         )
//         .assert_success();

//     let alice_sale = e.get_sale(0, Some(alice.valid_account_id()));
//     assert_eq!(alice_sale.in_token_paid_unclaimed.0, 0);
//     assert_eq!(
//         e.get_treasury_balances(),
//         vec![(e.w_near.account_id.clone(), to_yocto("2"))]
//     );

//     e.near.borrow_runtime_mut().cur_block.block_timestamp =
//         sale.start_time.0 + sale.duration.0 * 3 / 4;

//     let bobs_sale = e.get_sale(0, Some(bob.valid_account_id()));
//     e.assert_sale_eq(
//         &bobs_sale,
//         PartialSale {
//             out_tokens: vec![PartialOutToken {
//                 remaining: (sale_amount / 4).into(),
//                 distributed: (sale_amount * 3 / 4).into(),
//                 treasury_unclaimed: None,
//             }],
//             in_token_remaining: to_yocto("2.5").into(),
//             in_token_paid_unclaimed: to_yocto("2.5").into(),
//             in_token_paid: to_yocto("4.5").into(),
//             total_shares: to_yocto("10").into(),
//             subscription: Some(SubscriptionOutput {
//                 claimed_out_balance: vec![(sale_amount / 4 * 99 / 100).into()],
//                 spent_in_balance: to_yocto("3").into(),
//                 remaining_in_balance: to_yocto("1").into(),
//                 unclaimed_out_balances: vec![(sale_amount * 7 / 20).into()],
//                 shares: to_yocto("4").into(),
//                 referral_id: None,
//             }),
//         },
//     );

//     bob.function_call(
//         e.skyward
//             .contract
//             .sale_withdraw_in_token(0, Some(to_yocto("2").into())),
//         BASE_GAS,
//         1,
//     )
//     .assert_success();

//     let bobs_sale = e.get_sale(0, Some(bob.valid_account_id()));
//     e.assert_sale_eq(
//         &bobs_sale,
//         PartialSale {
//             out_tokens: vec![PartialOutToken {
//                 remaining: (sale_amount / 4).into(),
//                 distributed: (sale_amount * 3 / 4).into(),
//                 treasury_unclaimed: None,
//             }],
//             in_token_remaining: to_yocto("2.0").into(),
//             in_token_paid_unclaimed: to_yocto("0").into(),
//             in_token_paid: to_yocto("4.5").into(),
//             total_shares: to_yocto("8").into(),
//             subscription: Some(SubscriptionOutput {
//                 claimed_out_balance: vec![(sale_amount * 3 / 5 * 99 / 100).into()],
//                 spent_in_balance: to_yocto("3").into(),
//                 remaining_in_balance: to_yocto("0.5").into(),
//                 unclaimed_out_balances: vec![0.into()],
//                 shares: to_yocto("2").into(),
//                 referral_id: None,
//             }),
//         },
//     );

//     assert_eq!(
//         e.balances_of(bob),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("6.5")),
//             (
//                 e.skyward_token.account_id.clone(),
//                 sale_amount * 3 / 5 * 99 / 100
//             ),
//         ]
//     );

//     e.near.borrow_runtime_mut().cur_block.block_timestamp = sale.start_time.0 + sale.duration.0;

//     let bobs_sale = e.get_sale(0, Some(bob.valid_account_id()));
//     e.assert_sale_eq(
//         &bobs_sale,
//         PartialSale {
//             out_tokens: vec![PartialOutToken {
//                 remaining: 0.into(),
//                 distributed: sale_amount.into(),
//                 treasury_unclaimed: None,
//             }],
//             in_token_remaining: to_yocto("0").into(),
//             in_token_paid_unclaimed: to_yocto("2").into(),
//             in_token_paid: to_yocto("6.5").into(),
//             total_shares: to_yocto("8").into(),
//             subscription: Some(SubscriptionOutput {
//                 claimed_out_balance: vec![(sale_amount * 3 / 5 * 99 / 100).into()],
//                 spent_in_balance: to_yocto("3.5").into(),
//                 remaining_in_balance: to_yocto("0").into(),
//                 unclaimed_out_balances: vec![(sale_amount * 1 / 16).into()],
//                 shares: to_yocto("2").into(),
//                 referral_id: None,
//             }),
//         },
//     );

//     let alice_sale = e.get_sale(0, Some(alice.valid_account_id()));
//     assert_eq!(
//         alice_sale.subscription,
//         Some(SubscriptionOutput {
//             claimed_out_balance: vec![to_yocto("0").into()],
//             spent_in_balance: to_yocto("3").into(),
//             remaining_in_balance: to_yocto("0").into(),
//             unclaimed_out_balances: vec![(sale_amount * 27 / 80).into()],
//             shares: to_yocto("6").into(),
//             referral_id: None
//         })
//     );

//     assert_eq!(
//         e.get_treasury_balances(),
//         vec![(e.w_near.account_id.clone(), to_yocto("4.5"))]
//     );

//     assert_eq!(
//         e.skyward_circulating_supply(),
//         sale_amount - sale_amount * 3 / 5 / 100
//     );

//     alice
//         .function_call(e.skyward.contract.sale_claim_out_tokens(0), BASE_GAS, 0)
//         .assert_success();

//     assert_eq!(
//         e.balances_of(alice),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("7")),
//             (
//                 e.skyward_token.account_id.clone(),
//                 sale_amount * 27 / 80 * 99 / 100
//             ),
//         ]
//     );
//     assert_eq!(
//         e.skyward_circulating_supply(),
//         sale_amount - sale_amount * 3 / 5 / 100 - sale_amount * 27 / 80 / 100
//     );
//     assert_eq!(
//         e.get_treasury_balances(),
//         vec![(e.w_near.account_id.clone(), to_yocto("6.5"))]
//     );
//     assert_eq!(
//         e.balances_of(bob),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("6.5")),
//             (
//                 e.skyward_token.account_id.clone(),
//                 sale_amount * 3 / 5 * 99 / 100
//             ),
//         ]
//     );
//     let alice_sale = e.get_sale(0, Some(alice.valid_account_id()));
//     assert_eq!(alice_sale.total_shares.0, to_yocto("2"));
//     assert_eq!(alice_sale.subscription, None);

//     let bobs_sale = e.get_sale(0, Some(bob.valid_account_id()));
//     e.assert_sale_eq(
//         &bobs_sale,
//         PartialSale {
//             out_tokens: vec![PartialOutToken {
//                 remaining: 0.into(),
//                 distributed: sale_amount.into(),
//                 treasury_unclaimed: None,
//             }],
//             in_token_remaining: to_yocto("0").into(),
//             in_token_paid_unclaimed: to_yocto("0").into(),
//             in_token_paid: to_yocto("6.5").into(),
//             total_shares: to_yocto("2").into(),
//             subscription: Some(SubscriptionOutput {
//                 claimed_out_balance: vec![(sale_amount * 3 / 5 * 99 / 100).into()],
//                 spent_in_balance: to_yocto("3.5").into(),
//                 remaining_in_balance: to_yocto("0").into(),
//                 unclaimed_out_balances: vec![(sale_amount * 1 / 16).into()],
//                 shares: to_yocto("2").into(),
//                 referral_id: None,
//             }),
//         },
//     );

//     bob.function_call(e.skyward.contract.sale_claim_out_tokens(0), BASE_GAS, 0)
//         .assert_success();

//     let bobs_sale = e.get_sale(0, Some(bob.valid_account_id()));
//     e.assert_sale_eq(
//         &bobs_sale,
//         PartialSale {
//             out_tokens: vec![PartialOutToken {
//                 remaining: 0.into(),
//                 distributed: sale_amount.into(),
//                 treasury_unclaimed: None,
//             }],
//             in_token_remaining: to_yocto("0").into(),
//             in_token_paid_unclaimed: to_yocto("0").into(),
//             in_token_paid: to_yocto("6.5").into(),
//             total_shares: to_yocto("0").into(),
//             subscription: None,
//         },
//     );

//     assert_eq!(
//         e.skyward_circulating_supply(),
//         sale_amount - sale_amount / 100
//     );

//     assert_eq!(
//         e.get_treasury_balances(),
//         vec![(e.w_near.account_id.clone(), to_yocto("6.5"))]
//     );
//     assert_eq!(
//         e.balances_of(bob),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("6.5")),
//             (
//                 e.skyward_token.account_id.clone(),
//                 sale_amount * 53 / 80 * 99 / 100
//             ),
//         ]
//     );
// }

// #[test]
// fn test_skyward_sale_ref() {
//     let e = Env::init_with_schedule(4, vec![]);
//     let alice = e.users.get(0).unwrap();
//     let bob = e.users.get(1).unwrap();
//     let charlie = e.users.get(2).unwrap();
//     let danny = e.users.get(3).unwrap();

//     let sale_amount = 10000 * SKYWARD_TOKEN_BASE;
//     e.skyward_dao
//         .call(
//             e.skyward_token.account_id.clone(),
//             "ft_transfer",
//             &json!({
//                 "receiver_id": SKYWARD_ID,
//                 "amount": U128::from(sale_amount),
//             })
//             .to_string()
//             .into_bytes(),
//             BASE_GAS,
//             1,
//         )
//         .assert_success();
//     assert_eq!(
//         e.get_token_balance(&e.skyward_token, &e.skyward.user_account),
//         sale_amount
//     );

//     let sale = e.sale_create_with_ref(&e.skyward.user_account, &[(&e.skyward_token, sale_amount)]);

//     assert_eq!(e.skyward_circulating_supply(), 0);

//     e.register_skyward_token(alice);

//     bob.function_call(
//         e.skyward.contract.sale_deposit_in_token(
//             sale.sale_id,
//             to_yocto("4").into(),
//             Some(alice.valid_account_id()),
//         ),
//         BASE_GAS,
//         to_yocto("0.01"),
//     )
//     .assert_success();

//     charlie
//         .function_call(
//             e.skyward.contract.sale_deposit_in_token(
//                 sale.sale_id,
//                 to_yocto("4").into(),
//                 Some(e.near.valid_account_id()),
//             ),
//             BASE_GAS,
//             to_yocto("0.01"),
//         )
//         .assert_success();

//     danny
//         .function_call(
//             e.skyward
//                 .contract
//                 .sale_deposit_in_token(sale.sale_id, to_yocto("8").into(), None),
//             BASE_GAS,
//             to_yocto("0.01"),
//         )
//         .assert_success();

//     e.near.borrow_runtime_mut().cur_block.block_timestamp = sale.start_time.0 + sale.duration.0;

//     bob.function_call(e.skyward.contract.sale_claim_out_tokens(0), BASE_GAS, 0)
//         .assert_success();

//     charlie
//         .function_call(e.skyward.contract.sale_claim_out_tokens(0), BASE_GAS, 0)
//         .assert_success();

//     danny
//         .function_call(e.skyward.contract.sale_claim_out_tokens(0), BASE_GAS, 0)
//         .assert_success();

//     assert_eq!(
//         e.balances_of(alice),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("10")),
//             (e.skyward_token.account_id.clone(), sale_amount / 4 / 200),
//         ]
//     );

//     assert_eq!(
//         e.balances_of(bob),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("6")),
//             (
//                 e.skyward_token.account_id.clone(),
//                 sale_amount / 4 * 199 / 200
//             ),
//         ]
//     );

//     assert_eq!(
//         e.balances_of(charlie),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("6")),
//             (
//                 e.skyward_token.account_id.clone(),
//                 sale_amount / 4 * 199 / 200
//             ),
//         ]
//     );

//     assert_eq!(
//         e.balances_of(danny),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("2")),
//             (
//                 e.skyward_token.account_id.clone(),
//                 sale_amount / 2 * 99 / 100
//             ),
//         ]
//     );

//     assert_eq!(
//         e.skyward_circulating_supply(),
//         sale_amount - sale_amount / 4 / 200 - sale_amount / 2 / 100
//     );
// }

// #[test]
// fn test_circulating_supply() {
//     let e = Env::init_with_schedule(
//         0,
//         vec![
//             VestingIntervalInput {
//                 start_timestamp: GENESIS_TIME + MONTH,
//                 end_timestamp: GENESIS_TIME + MONTH * 2,
//                 amount: U128(10000 * SKYWARD_TOKEN_BASE),
//             },
//             VestingIntervalInput {
//                 start_timestamp: GENESIS_TIME + MONTH * 6,
//                 end_timestamp: GENESIS_TIME + MONTH * 12,
//                 amount: U128(90000 * SKYWARD_TOKEN_BASE),
//             },
//         ],
//     );

//     let total_sale_amount = 900000 * SKYWARD_TOKEN_BASE;
//     e.skyward_dao
//         .call(
//             e.skyward_token.account_id.clone(),
//             "ft_transfer",
//             &json!({
//                 "receiver_id": SKYWARD_ID,
//                 "amount": U128::from(total_sale_amount),
//             })
//             .to_string()
//             .into_bytes(),
//             BASE_GAS,
//             1,
//         )
//         .assert_success();
//     assert_eq!(
//         e.get_token_balance(&e.skyward_token, &e.skyward.user_account),
//         total_sale_amount
//     );

//     let sales_input = vec![
//         (5, WEEK),
//         (20, WEEK * 3),
//         (20, MONTH + WEEK * 3),
//         (15, 2 * MONTH + WEEK * 3),
//         (10, 3 * MONTH + WEEK * 3),
//         (10, 4 * MONTH + WEEK * 3),
//         (10, 5 * MONTH + WEEK * 3),
//     ];

//     let sales: Vec<_> = sales_input
//         .iter()
//         .map(|&(amount, start_offset)| {
//             let sale_amount = amount * 10000 * SKYWARD_TOKEN_BASE;
//             e.sale_create_custom(
//                 &e.skyward.user_account,
//                 &[(&e.skyward_token, sale_amount)],
//                 to_nano(start_offset),
//                 to_nano(WEEK),
//                 None,
//                 None,
//             )
//         })
//         .collect();

//     assert_eq!(e.skyward_circulating_supply(), 0);

//     e.near.borrow_runtime_mut().cur_block.block_timestamp =
//         sales[0].start_time.0 + sales[0].duration.0 / 2;
//     assert_eq!(e.skyward_circulating_supply(), 25000 * SKYWARD_TOKEN_BASE);

//     e.near.borrow_runtime_mut().cur_block.block_timestamp =
//         sales[0].start_time.0 + sales[0].duration.0;
//     assert_eq!(e.skyward_circulating_supply(), 50000 * SKYWARD_TOKEN_BASE);

//     e.near.borrow_runtime_mut().cur_block.block_timestamp = to_nano(GENESIS_TIME + WEEK * 2);
//     assert_eq!(e.skyward_circulating_supply(), 50000 * SKYWARD_TOKEN_BASE);

//     e.near.borrow_runtime_mut().cur_block.block_timestamp = sales[1].start_time.0;
//     assert_eq!(e.skyward_circulating_supply(), 50000 * SKYWARD_TOKEN_BASE);

//     e.near.borrow_runtime_mut().cur_block.block_timestamp =
//         sales[1].start_time.0 + sales[1].duration.0 / 2;
//     assert_eq!(e.skyward_circulating_supply(), 150000 * SKYWARD_TOKEN_BASE);

//     e.near.borrow_runtime_mut().cur_block.block_timestamp =
//         sales[1].start_time.0 + sales[1].duration.0;
//     assert_eq!(e.skyward_circulating_supply(), 250000 * SKYWARD_TOKEN_BASE);

//     e.near.borrow_runtime_mut().cur_block.block_timestamp =
//         to_nano(GENESIS_TIME + MONTH + MONTH / 2);
//     assert_eq!(e.skyward_circulating_supply(), 255000 * SKYWARD_TOKEN_BASE);

//     e.near.borrow_runtime_mut().cur_block.block_timestamp = to_nano(GENESIS_TIME + MONTH * 2);
//     assert_eq!(e.skyward_circulating_supply(), 460000 * SKYWARD_TOKEN_BASE);

//     e.near.borrow_runtime_mut().cur_block.block_timestamp =
//         sales[3].start_time.0 + sales[3].duration.0 / 2;
//     assert_eq!(e.skyward_circulating_supply(), 535000 * SKYWARD_TOKEN_BASE);

//     e.near.borrow_runtime_mut().cur_block.block_timestamp = to_nano(GENESIS_TIME + MONTH * 6);
//     assert_eq!(e.skyward_circulating_supply(), 910000 * SKYWARD_TOKEN_BASE);

//     e.near.borrow_runtime_mut().cur_block.block_timestamp = to_nano(GENESIS_TIME + MONTH * 9);
//     assert_eq!(e.skyward_circulating_supply(), 955000 * SKYWARD_TOKEN_BASE);

//     e.near.borrow_runtime_mut().cur_block.block_timestamp = to_nano(GENESIS_TIME + MONTH * 12);
//     assert_eq!(e.skyward_circulating_supply(), SKYWARD_TOTAL_SUPPLY);
// }

// #[test]
// fn test_skyward_sale_half_empty() {
//     let e = Env::init_with_schedule(1, vec![]);
//     let alice = e.users.get(0).unwrap();

//     let sale_amount = 10000 * SKYWARD_TOKEN_BASE;
//     e.skyward_dao
//         .call(
//             e.skyward_token.account_id.clone(),
//             "ft_transfer",
//             &json!({
//                 "receiver_id": SKYWARD_ID,
//                 "amount": U128::from(sale_amount),
//             })
//             .to_string()
//             .into_bytes(),
//             BASE_GAS,
//             1,
//         )
//         .assert_success();
//     assert_eq!(
//         e.get_token_balance(&e.skyward_token, &e.skyward.user_account),
//         sale_amount
//     );

//     let sale = e.sale_create_with_ref(&e.skyward.user_account, &[(&e.skyward_token, sale_amount)]);

//     assert_eq!(e.skyward_circulating_supply(), 0);

//     alice
//         .function_call(
//             e.skyward
//                 .contract
//                 .sale_deposit_in_token(sale.sale_id, to_yocto("4").into(), None),
//             BASE_GAS,
//             to_yocto("0.01"),
//         )
//         .assert_success();

//     e.near.borrow_runtime_mut().cur_block.block_timestamp = sale.start_time.0 + sale.duration.0 / 2;

//     alice
//         .function_call(
//             e.skyward.contract.sale_withdraw_in_token(0, None),
//             BASE_GAS,
//             1,
//         )
//         .assert_success();

//     assert_eq!(
//         e.balances_of(alice),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("8")),
//             (
//                 e.skyward_token.account_id.clone(),
//                 sale_amount / 2 * 99 / 100
//             ),
//         ]
//     );

//     assert_eq!(
//         e.skyward_circulating_supply(),
//         sale_amount / 2 - sale_amount / 2 / 100
//     );

//     e.near.borrow_runtime_mut().cur_block.block_timestamp = sale.start_time.0 + sale.duration.0;

//     assert_eq!(
//         e.skyward_circulating_supply(),
//         sale_amount - sale_amount / 2 / 100
//     );

//     alice
//         .function_call(
//             e.skyward.contract.sale_distribute_unclaimed_tokens(0),
//             BASE_GAS,
//             0,
//         )
//         .assert_success();

//     assert_eq!(
//         e.skyward_circulating_supply(),
//         sale_amount / 2 - sale_amount / 2 / 100
//     );
// }

// #[test]
// fn test_regular_sale_half_empty() {
//     let e = Env::init_with_schedule(2, vec![]);
//     let alice = e.users.get(0).unwrap();
//     let bob = e.users.get(1).unwrap();

//     let token1 = e.deploy_ft(&alice.account_id, TOKEN1_ID);
//     e.register_and_deposit(&alice, &token1, to_yocto("10000"));

//     let sale_amount = to_yocto("4000");
//     let sale = e.sale_create_with_ref(alice, &[(&token1, sale_amount)]);

//     assert_eq!(e.skyward_circulating_supply(), 0);

//     bob.function_call(
//         e.skyward
//             .contract
//             .sale_deposit_in_token(sale.sale_id, to_yocto("4").into(), None),
//         BASE_GAS,
//         to_yocto("0.01"),
//     )
//     .assert_success();

//     e.near.borrow_runtime_mut().cur_block.block_timestamp = sale.start_time.0 + sale.duration.0 / 2;

//     bob.function_call(
//         e.skyward.contract.sale_withdraw_in_token(0, None),
//         BASE_GAS,
//         1,
//     )
//     .assert_success();

//     assert_eq!(
//         e.balances_of(bob),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("8")),
//             (
//                 token1.account_id.clone(),
//                 sale_amount * 99 / 100 / 2 * 99 / 100
//             ),
//         ]
//     );

//     assert_eq!(
//         e.balances_of(alice),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("11.98")),
//             (
//                 token1.account_id.clone(),
//                 to_yocto("6000") + sale_amount * 99 / 100 / 2 / 100
//             ),
//         ]
//     );

//     e.near.borrow_runtime_mut().cur_block.block_timestamp = sale.start_time.0 + sale.duration.0;

//     alice
//         .function_call(
//             e.skyward.contract.sale_distribute_unclaimed_tokens(0),
//             BASE_GAS,
//             0,
//         )
//         .assert_success();

//     assert_eq!(
//         e.balances_of(alice),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("11.98")),
//             (
//                 token1.account_id.clone(),
//                 to_yocto("6000") + sale_amount / 2 + sale_amount * 99 / 100 / 2 / 100
//             ),
//         ]
//     );
// }

// #[test]
// fn test_regular_sale_join_in_the_middle() {
//     let e = Env::init_with_schedule(2, vec![]);
//     let alice = e.users.get(0).unwrap();
//     let bob = e.users.get(1).unwrap();

//     let token1 = e.deploy_ft(&alice.account_id, TOKEN1_ID);
//     e.register_and_deposit(&alice, &token1, to_yocto("10000"));

//     let sale_amount = to_yocto("4000");
//     let sale = e.sale_create(alice, &[(&token1, sale_amount)]);

//     assert_eq!(e.skyward_circulating_supply(), 0);

//     e.near.borrow_runtime_mut().cur_block.block_timestamp = sale.start_time.0 + sale.duration.0 / 2;

//     bob.function_call(
//         e.skyward
//             .contract
//             .sale_deposit_in_token(sale.sale_id, to_yocto("4").into(), None),
//         BASE_GAS,
//         to_yocto("0.01"),
//     )
//     .assert_success();

//     e.near.borrow_runtime_mut().cur_block.block_timestamp = sale.start_time.0 + sale.duration.0;

//     bob.function_call(e.skyward.contract.sale_claim_out_tokens(0), BASE_GAS, 0)
//         .assert_success();

//     assert_eq!(
//         e.balances_of(bob),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("6")),
//             (token1.account_id.clone(), sale_amount * 99 / 100),
//         ]
//     );

//     assert_eq!(
//         e.balances_of(alice),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("13.96")),
//             (token1.account_id.clone(), to_yocto("6000")),
//         ]
//     );
// }

// #[test]
// fn test_permissions_sale() {
//     let e = Env::init_with_schedule(2, vec![]);
//     let alice = e.users.get(0).unwrap();
//     let bob = e.users.get(1).unwrap();

//     let token1 = e.deploy_ft(&alice.account_id, TOKEN1_ID);
//     e.register_and_deposit(&alice, &token1, to_yocto("10000"));

//     let sale_amount = to_yocto("4000");
//     let sale = e.sale_create_custom(
//         alice,
//         &[(&token1, sale_amount)],
//         to_nano(WEEK) + BLOCK_DURATION * 15,
//         BLOCK_DURATION * 60,
//         Some(e.permissions_contract.valid_account_id()),
//         None,
//     );

//     assert!(!bob
//         .function_call(
//             e.skyward
//                 .contract
//                 .sale_deposit_in_token(sale.sale_id, to_yocto("4").into(), None),
//             BASE_GAS,
//             to_yocto("0.01"),
//         )
//         .is_ok());

//     let initial_balance = e.skyward.user_account.account().unwrap().amount;
//     let result: bool = bob
//         .function_call(
//             e.skyward
//                 .contract
//                 .sale_deposit_in_token(sale.sale_id, to_yocto("4").into(), None),
//             TON_OF_GAS,
//             to_yocto("1"),
//         )
//         .unwrap_json();
//     assert!(!result);
//     let end_balance = e.skyward.user_account.account().unwrap().amount;
//     assert!(end_balance - initial_balance < to_yocto("0.01"));

//     e.skyward_dao
//         .call(
//             e.permissions_contract.account_id.clone(),
//             "approve",
//             &json!({
//                 "account_id": bob.valid_account_id()
//             })
//             .to_string()
//             .into_bytes(),
//             BASE_GAS,
//             0,
//         )
//         .assert_success();

//     let initial_balance = e.skyward.user_account.account().unwrap().amount;
//     let result: bool = bob
//         .function_call(
//             e.skyward
//                 .contract
//                 .sale_deposit_in_token(sale.sale_id, to_yocto("4").into(), None),
//             TON_OF_GAS,
//             to_yocto("1"),
//         )
//         .unwrap_json();
//     assert!(result);
//     let end_balance = e.skyward.user_account.account().unwrap().amount;
//     assert!(end_balance - initial_balance < to_yocto("0.01"));

//     // Already approved, so don't need ton of gas.
//     bob.function_call(
//         e.skyward
//             .contract
//             .sale_deposit_in_token(sale.sale_id, to_yocto("4").into(), None),
//         BASE_GAS,
//         to_yocto("0.01"),
//     )
//     .assert_success();

//     e.near.borrow_runtime_mut().cur_block.block_timestamp = sale.start_time.0 + sale.duration.0;

//     bob.function_call(e.skyward.contract.sale_claim_out_tokens(0), BASE_GAS, 0)
//         .assert_success();

//     assert_eq!(
//         e.balances_of(bob),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("2")),
//             (token1.account_id.clone(), sale_amount * 99 / 100),
//         ]
//     );
// }

// #[test]
// fn test_invalid_permissions_sale() {
//     let e = Env::init_with_schedule(2, vec![]);
//     let alice = e.users.get(0).unwrap();
//     let bob = e.users.get(1).unwrap();

//     let token1 = e.deploy_ft(&alice.account_id, TOKEN1_ID);
//     e.register_and_deposit(&alice, &token1, to_yocto("10000"));

//     let sale_amount = to_yocto("4000");
//     let sale = e.sale_create_custom(
//         alice,
//         &[(&token1, sale_amount)],
//         to_nano(WEEK) + BLOCK_DURATION * 15,
//         BLOCK_DURATION * 60,
//         Some(e.near.valid_account_id()),
//         None,
//     );

//     assert!(!bob
//         .function_call(
//             e.skyward
//                 .contract
//                 .sale_deposit_in_token(sale.sale_id, to_yocto("4").into(), None),
//             BASE_GAS,
//             to_yocto("0.01"),
//         )
//         .is_ok());

//     let initial_balance = e.skyward.user_account.account().unwrap().amount;
//     let result: bool = bob
//         .function_call(
//             e.skyward
//                 .contract
//                 .sale_deposit_in_token(sale.sale_id, to_yocto("4").into(), None),
//             TON_OF_GAS,
//             to_yocto("1"),
//         )
//         .unwrap_json();
//     assert!(!result);
//     let end_balance = e.skyward.user_account.account().unwrap().amount;
//     assert!(end_balance - initial_balance < to_yocto("0.01"));

//     e.near.borrow_runtime_mut().cur_block.block_timestamp = sale.start_time.0 + sale.duration.0;

//     alice
//         .function_call(
//             e.skyward.contract.sale_distribute_unclaimed_tokens(0),
//             BASE_GAS,
//             0,
//         )
//         .assert_success();

//     assert_eq!(
//         e.balances_of(alice),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("10")),
//             (token1.account_id.clone(), to_yocto("10000")),
//         ]
//     );
// }
