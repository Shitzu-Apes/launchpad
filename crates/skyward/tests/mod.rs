mod util;

use near_contract_standards::fungible_token::metadata::{FungibleTokenMetadata, FT_METADATA_SPEC};
use near_sdk::{json_types::U128, serde_json::json, Gas, NearToken};
use near_workspaces::{
    types::{KeyType, SecretKey},
    AccountId,
};
use skyward::{
    SaleInput, SaleInputOutToken, SaleOutput, SaleOutputOutToken, SubscriptionOutput,
    VestingIntervalInput,
};
use util::*;

const SKYWARD_WASM_BYTES: &[u8] = include_bytes!("../../../res/skyward.wasm");
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

const GENESIS_TIME: u32 = 1_600_000_000;
const DAY: u32 = 24 * 60 * 60;
const WEEK: u32 = 7 * DAY;
const MONTH: u32 = 30 * DAY;
const TON_OF_GAS: Gas = Gas::from_tgas(200);
const SKYWARD_TOKEN_DECIMALS: u8 = 18;
const SKYWARD_TOKEN_BASE: u128 = 10u128.pow(SKYWARD_TOKEN_DECIMALS as u32);
const SKYWARD_TOTAL_SUPPLY: u128 = 1_000_000 * SKYWARD_TOKEN_BASE;
const ONE_NEAR: u128 = 10u128.pow(24);
const LISTING_FEE_NEAR: NearToken = NearToken::from_near(10);
const DEFAULT_TOTAL_SUPPLY: u128 = 1_000_000_000 * ONE_NEAR;

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
    pub treasury_unclaimed: Option<U128>,
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

// #[test]
// fn test_ft_transfer_call_donate() {
//     let e = Env::init(1);
//     let alice = e.users.get(0).unwrap();

//     let token1 = e.deploy_ft(&alice.account_id, TOKEN1_ID);
//     e.register_and_deposit(&alice, &token1, to_yocto("10000"));

//     assert_eq!(
//         e.get_treasury_balances(),
//         vec![
//             (e.w_near.account_id.clone(), 0),
//             (token1.account_id.clone(), 0)
//         ]
//     );

//     alice
//         .call(
//             token1.account_id.clone(),
//             "ft_transfer_call",
//             &json!({
//                 "receiver_id": e.skyward.user_account.valid_account_id(),
//                 "amount": U128::from(to_yocto("50000")),
//                 "msg": "\"DonateToTreasury\"",
//             })
//             .to_string()
//             .into_bytes(),
//             TON_OF_GAS,
//             1,
//         )
//         .assert_success();

//     assert_eq!(
//         e.get_treasury_balances(),
//         vec![
//             (e.w_near.account_id.clone(), 0),
//             (token1.account_id.clone(), to_yocto("50000"))
//         ]
//     );
// }

// #[test]
// fn test_wrap_extra_near() {
//     let e = Env::init(0);

//     assert_eq!(e.get_treasury_balances(), vec![]);

//     e.root
//         .transfer(e.skyward.user_account.account_id.clone(), to_yocto("9000"))
//         .assert_success();

//     assert_eq!(e.get_token_balance(&e.w_near, &e.skyward.user_account), 0);

//     let initial_balance = e.skyward.user_account.account().unwrap().amount;

//     let res = e
//         .near
//         .function_call(e.skyward.contract.wrap_extra_near(), TON_OF_GAS, 0);
//     res.assert_success();
//     let res: bool = res.unwrap_json();
//     assert!(res);

//     let near_spent = initial_balance - e.skyward.user_account.account().unwrap().amount;
//     assert!(near_spent > to_yocto("9000"));

//     let w_near_balance = e.get_treasury_balances()[0].1;
//     assert!(w_near_balance > to_yocto("9000"));
//     assert_eq!(
//         e.get_token_balance(&e.w_near, &e.skyward.user_account),
//         w_near_balance
//     );

//     assert!(!e
//         .near
//         .function_call(e.skyward.contract.wrap_extra_near(), TON_OF_GAS, 0)
//         .is_ok());

//     e.root
//         .transfer(e.skyward.user_account.account_id.clone(), to_yocto("10.1"))
//         .assert_success();

//     let initial_balance = e.skyward.user_account.account().unwrap().amount;

//     let res = e
//         .near
//         .function_call(e.skyward.contract.wrap_extra_near(), TON_OF_GAS, 0);
//     res.assert_success();
//     let res: bool = res.unwrap_json();
//     assert!(res);

//     let near_spent = initial_balance - e.skyward.user_account.account().unwrap().amount;
//     assert!(near_spent > to_yocto("10"));

//     let w_near_balance_addition = e.get_treasury_balances()[0].1 - w_near_balance;
//     assert!(w_near_balance_addition > to_yocto("10"));
//     assert_eq!(
//         e.get_token_balance(&e.w_near, &e.skyward.user_account),
//         w_near_balance + w_near_balance_addition
//     );
// }

// #[test]
// fn test_create_sale() {
//     let e = Env::init(1);
//     let alice = e.users.get(0).unwrap();

//     let token1 = e.deploy_ft(&alice.account_id, TOKEN1_ID);
//     e.register_and_deposit(&alice, &token1, to_yocto("10000"));

//     let sale = e.sale_create(alice, &[(&token1, to_yocto("4000"))]);

//     assert_eq!(
//         sale,
//         SaleOutput {
//             sale_id: 0,
//             title: TITLE.to_string(),
//             url: None,
//             permissions_contract_id: None,
//             owner_id: alice.account_id.clone(),
//             out_tokens: vec![SaleOutputOutToken {
//                 token_account_id: token1.account_id.clone(),
//                 remaining: to_yocto("4000").into(),
//                 distributed: 0.into(),
//                 treasury_unclaimed: Some(0.into()),
//                 referral_bpt: None
//             }],
//             in_token_account_id: e.w_near.account_id.clone(),
//             in_token_remaining: U128(0),
//             in_token_paid_unclaimed: U128(0),
//             in_token_paid: U128(0),
//             total_shares: U128(0),
//             start_time: (to_nano(GENESIS_TIME + WEEK) + BLOCK_DURATION * 15).into(),
//             duration: (BLOCK_DURATION * 60).into(),
//             remaining_duration: (BLOCK_DURATION * 60).into(),
//             subscription: None,
//             current_time: to_nano(GENESIS_TIME).into(),
//             current_block_height: alice.borrow_runtime().current_block().block_height,
//             start_block_height: sale.start_block_height,
//             end_block_height: None
//         },
//     );

//     assert_eq!(
//         e.balances_of(alice),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("10")),
//             (token1.account_id.clone(), to_yocto("6000")),
//         ]
//     );
// }

// #[test]
// fn test_join_sale() {
//     let e = Env::init(2);
//     let alice = e.users.get(0).unwrap();
//     let bob = e.users.get(1).unwrap();

//     let token1 = e.deploy_ft(&alice.account_id, TOKEN1_ID);
//     e.register_and_deposit(&alice, &token1, to_yocto("10000"));

//     let sale = e.sale_create(alice, &[(&token1, to_yocto("3600"))]);

//     bob.function_call(
//         e.skyward
//             .contract
//             .sale_deposit_in_token(sale.sale_id, to_yocto("4").into(), None),
//         BASE_GAS,
//         to_yocto("0.01"),
//     )
//     .assert_success();

//     let bobs_sale = e.get_sale(0, Some(bob.valid_account_id()));
//     assert_eq!(bobs_sale.in_token_remaining.0, to_yocto("4"));
//     assert_eq!(bobs_sale.total_shares.0, to_yocto("4"));
//     assert_eq!(
//         bobs_sale.subscription,
//         Some(SubscriptionOutput {
//             claimed_out_balance: vec![to_yocto("0").into()],
//             spent_in_balance: to_yocto("0").into(),
//             remaining_in_balance: to_yocto("4").into(),
//             unclaimed_out_balances: vec![U128(0)],
//             shares: to_yocto("4").into(),
//             referral_id: None
//         })
//     );

//     assert_eq!(
//         e.balances_of(bob),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("6")),
//             (token1.account_id.clone(), 0),
//         ]
//     );

//     e.near.borrow_runtime_mut().cur_block.block_timestamp = sale.start_time.0 + sale.duration.0 / 2;

//     let bobs_sale = e.get_sale(0, Some(bob.valid_account_id()));
//     e.assert_sale_eq(
//         &bobs_sale,
//         PartialSale {
//             out_tokens: vec![PartialOutToken {
//                 remaining: to_yocto("1800").into(),
//                 distributed: to_yocto("1800").into(),
//                 treasury_unclaimed: Some(to_yocto("18").into()),
//             }],
//             in_token_remaining: to_yocto("2").into(),
//             in_token_paid_unclaimed: to_yocto("2").into(),
//             in_token_paid: to_yocto("2").into(),
//             total_shares: to_yocto("4").into(),
//             subscription: Some(SubscriptionOutput {
//                 claimed_out_balance: vec![to_yocto("0").into()],
//                 spent_in_balance: to_yocto("2").into(),
//                 remaining_in_balance: to_yocto("2").into(),
//                 unclaimed_out_balances: vec![to_yocto("1782").into()],
//                 shares: to_yocto("4").into(),
//                 referral_id: None,
//             }),
//         },
//     );

//     e.near.borrow_runtime_mut().cur_block.block_timestamp = sale.start_time.0 + sale.duration.0;

//     let bobs_sale = e.get_sale(0, Some(bob.valid_account_id()));

//     e.assert_sale_eq(
//         &bobs_sale,
//         PartialSale {
//             out_tokens: vec![PartialOutToken {
//                 remaining: 0.into(),
//                 distributed: to_yocto("3600").into(),
//                 treasury_unclaimed: Some(to_yocto("36").into()),
//             }],
//             in_token_remaining: to_yocto("0").into(),
//             in_token_paid_unclaimed: to_yocto("4").into(),
//             in_token_paid: to_yocto("4").into(),
//             total_shares: to_yocto("4").into(),
//             subscription: Some(SubscriptionOutput {
//                 claimed_out_balance: vec![to_yocto("0").into()],
//                 spent_in_balance: to_yocto("4").into(),
//                 remaining_in_balance: to_yocto("0").into(),
//                 unclaimed_out_balances: vec![to_yocto("3564").into()],
//                 shares: to_yocto("4").into(),
//                 referral_id: None,
//             }),
//         },
//     );

//     assert_eq!(
//         e.get_treasury_balances(),
//         vec![
//             (e.w_near.account_id.clone(), 0),
//             (token1.account_id.clone(), 0),
//         ]
//     );

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
//             (e.w_near.account_id.clone(), to_yocto("13.96")),
//             (token1.account_id.clone(), to_yocto("6400")),
//         ]
//     );
//     assert_eq!(
//         e.get_treasury_balances(),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("0.04")),
//             (token1.account_id.clone(), to_yocto("36")),
//         ]
//     );
//     assert_eq!(
//         e.balances_of(bob),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("6")),
//             (token1.account_id.clone(), 0),
//         ]
//     );

//     let bobs_sale = e.get_sale(0, Some(bob.valid_account_id()));

//     e.assert_sale_eq(
//         &bobs_sale,
//         PartialSale {
//             out_tokens: vec![PartialOutToken {
//                 remaining: 0.into(),
//                 distributed: to_yocto("3600").into(),
//                 treasury_unclaimed: Some(0.into()),
//             }],
//             in_token_remaining: to_yocto("0").into(),
//             in_token_paid_unclaimed: to_yocto("0").into(),
//             in_token_paid: to_yocto("4").into(),
//             total_shares: to_yocto("4").into(),
//             subscription: Some(SubscriptionOutput {
//                 claimed_out_balance: vec![to_yocto("0").into()],
//                 spent_in_balance: to_yocto("4").into(),
//                 remaining_in_balance: to_yocto("0").into(),
//                 unclaimed_out_balances: vec![to_yocto("3564").into()],
//                 shares: to_yocto("4").into(),
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
//                 distributed: to_yocto("3600").into(),
//                 treasury_unclaimed: Some(0.into()),
//             }],
//             in_token_remaining: to_yocto("0").into(),
//             in_token_paid_unclaimed: to_yocto("0").into(),
//             in_token_paid: to_yocto("4").into(),
//             total_shares: to_yocto("0").into(),
//             subscription: None,
//         },
//     );

//     assert_eq!(
//         e.get_treasury_balances(),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("0.04")),
//             (token1.account_id.clone(), to_yocto("36")),
//         ]
//     );
//     assert_eq!(
//         e.balances_of(bob),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("6")),
//             (token1.account_id.clone(), to_yocto("3564")),
//         ]
//     );
// }

// #[test]
// fn test_join_sale_with_referral() {
//     let e = Env::init(2);
//     let alice = e.users.get(0).unwrap();
//     let bob = e.users.get(1).unwrap();

//     let sale_amount = 10000 * SKYWARD_TOKEN_BASE;
//     e.register_and_deposit(&e.skyward_dao, &e.skyward_token, sale_amount * 2);

//     e.register_skyward_token(alice);

//     let sale = e.sale_create_with_ref(&e.skyward_dao, &[(&e.skyward_token, sale_amount)]);

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
//             in_token_paid_unclaimed: to_yocto("0").into(),
//             in_token_paid: to_yocto("0").into(),
//             total_shares: to_yocto("4").into(),
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
//         e.balances_of(alice),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("10")),
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
//             in_token_remaining: to_yocto("2").into(),
//             in_token_paid_unclaimed: to_yocto("2").into(),
//             in_token_paid: to_yocto("2").into(),
//             total_shares: to_yocto("4").into(),
//             subscription: Some(SubscriptionOutput {
//                 claimed_out_balance: vec![to_yocto("0").into()],
//                 spent_in_balance: to_yocto("2").into(),
//                 remaining_in_balance: to_yocto("2").into(),
//                 unclaimed_out_balances: vec![(sale_amount / 2).into()],
//                 shares: to_yocto("4").into(),
//                 referral_id: Some(alice.account_id.clone()),
//             }),
//         },
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
//             in_token_paid_unclaimed: to_yocto("4").into(),
//             in_token_paid: to_yocto("4").into(),
//             total_shares: to_yocto("4").into(),
//             subscription: Some(SubscriptionOutput {
//                 claimed_out_balance: vec![to_yocto("0").into()],
//                 spent_in_balance: to_yocto("4").into(),
//                 remaining_in_balance: to_yocto("0").into(),
//                 unclaimed_out_balances: vec![sale_amount.into()],
//                 shares: to_yocto("4").into(),
//                 referral_id: Some(alice.account_id.clone()),
//             }),
//         },
//     );

//     assert_eq!(
//         e.get_treasury_balances(),
//         vec![(e.w_near.account_id.clone(), 0),]
//     );

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
//             (e.w_near.account_id.clone(), to_yocto("3.96")),
//         ]
//     );

//     assert_eq!(
//         e.balances_of(alice),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("10")),
//             (e.skyward_token.account_id.clone(), 0),
//         ]
//     );
//     assert_eq!(
//         e.get_treasury_balances(),
//         vec![(e.w_near.account_id.clone(), to_yocto("0.04")),]
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
//             in_token_paid: to_yocto("4").into(),
//             total_shares: to_yocto("4").into(),
//             subscription: Some(SubscriptionOutput {
//                 claimed_out_balance: vec![to_yocto("0").into()],
//                 spent_in_balance: to_yocto("4").into(),
//                 remaining_in_balance: to_yocto("0").into(),
//                 unclaimed_out_balances: vec![sale_amount.into()],
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
//             in_token_paid: to_yocto("4").into(),
//             total_shares: to_yocto("0").into(),
//             subscription: None,
//         },
//     );

//     assert_eq!(
//         e.get_treasury_balances(),
//         vec![(e.w_near.account_id.clone(), to_yocto("0.04")),]
//     );
//     assert_eq!(
//         e.balances_of(alice),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("10")),
//             (e.skyward_token.account_id.clone(), sale_amount / 200),
//         ]
//     );
//     assert_eq!(
//         e.balances_of(bob),
//         vec![
//             (e.w_near.account_id.clone(), to_yocto("6")),
//             (e.skyward_token.account_id.clone(), sale_amount / 200 * 199),
//         ]
//     );
//     assert_eq!(e.skyward_circulating_supply(), SKYWARD_TOTAL_SUPPLY);
// }

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
