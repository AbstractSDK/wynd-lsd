use cosmwasm_std::{assert_approx_eq, ContractInfo, Decimal, Env, Order, Uint128};

use crate::{
    multitest::suite::SuiteBuilder,
    state::{BONDED, SUPPLY, UNBONDING},
    ContractError,
};
use test_case::test_case;

const DAY: u64 = 24 * HOUR;
const HOUR: u64 = 60 * MINUTE;
const MINUTE: u64 = 60;

#[test]
fn simple_liveness_slash() {
    let delegator = "delegator";

    let amount = 1_000_000u128;
    let mut suite = SuiteBuilder::new()
        .with_initial_balances(vec![(delegator, amount)])
        .build();

    suite.bond(delegator, amount).unwrap();

    // User now has the same amount of LSD tokens as they staked
    let lsd = suite.query_lsd_token().unwrap();
    let lsd_token_balance = suite.query_cw20_balance(delegator, &lsd).unwrap();

    // Wait until next epoch and reinvest to trigger delegation
    suite.update_time(23 * HOUR);
    suite.reinvest().unwrap();

    // Simulate liveness slash (0.1%)
    suite.slash("testvaloper1", Decimal::permille(1)).unwrap();
    suite.check_slash().unwrap();

    // check that storage is adjusted correctly
    let supply = SUPPLY.query(&suite.app.wrap(), suite.hub.clone()).unwrap();
    let bonded = BONDED.query(&suite.app.wrap(), suite.hub.clone()).unwrap();
    assert_eq!(supply.total_bonded.u128(), amount - amount / 1000);
    assert_eq!(
        bonded
            .iter()
            .find(|(val, _)| val == "testvaloper1")
            .unwrap()
            .1
            .u128(),
        amount - amount / 1000
    );
    let mut storage = suite.read_hub_storage();
    let supply = supply
        .cleanup_unbonding(
            &mut storage,
            &Env {
                block: suite.app.block_info(),
                transaction: None,
                contract: ContractInfo {
                    address: suite.hub.clone(),
                },
            },
        )
        .unwrap();
    assert_eq!(
        supply.tokens_per_share(
            suite
                .query_balance(suite.hub.as_str(), "FUN")
                .unwrap()
                .into()
        ),
        Decimal::permille(999)
    );

    // Unbond all of delegator's previously bonded tokens
    suite.unbond(delegator, &lsd, lsd_token_balance).unwrap();

    // Unbonding gets triggered every 28 / 7 = 4 days = 24 hours * 4 = 96 hours,
    // so we need to wait 5 epochs
    suite.update_time(5 * 23 * HOUR);
    suite.reinvest().unwrap();

    // wait until unbonding is complete
    suite.update_time(28 * DAY);
    suite.process_native_unbonding();

    // Claim delegator's tokens
    suite.claim(delegator).unwrap();

    // Verify delegator[0] has their native tokens back
    assert_eq!(
        suite.query_balance(delegator, "FUN").unwrap(),
        amount - amount / 1000
    );
}

#[test_case(28 * DAY; "exactly unbonding time")]
#[test_case(28 * DAY + MINUTE; "1 minute after unbonding should be done")]
#[test_case(27 * DAY + 23 * HOUR + 59 * MINUTE; "1 minute before unbonding should be done")]
fn timing_check(wait_time: u64) {
    let delegator = "delegator";

    let amount = 1_000_000u128;
    let mut suite = SuiteBuilder::new()
        .with_initial_balances(vec![(delegator, amount)])
        .build();

    suite.bond(delegator, amount).unwrap();

    // User now has the same amount of LSD tokens as they staked
    let lsd = suite.query_lsd_token().unwrap();
    let lsd_token_balance = suite.query_cw20_balance(delegator, &lsd).unwrap();

    // Wait until next epoch and reinvest to trigger delegation
    suite.update_time(23 * HOUR);
    suite.reinvest().unwrap();

    suite.unbond(delegator, &lsd, lsd_token_balance).unwrap();

    // Unbonding gets triggered every 28 / 7 = 4 days = 24 hours * 4 = 96 hours,
    // so we need to wait 5 more epochs after last reinvest
    suite.update_time(5 * 23 * HOUR);
    suite.reinvest().unwrap();

    // wait until the unbonding is almost complete
    suite.update_time(wait_time);

    // Simulate liveness slash (0.1%)
    suite.slash("testvaloper1", Decimal::permille(1)).unwrap();
    let err = suite.check_slash().unwrap_err();
    assert_eq!(ContractError::UnbondingTooClose {}, err.downcast().unwrap());
}

#[test]
fn pending_claims_slashed() {
    let delegator = "delegator";

    let amount = 1_000_000u128;
    let mut suite = SuiteBuilder::new()
        .with_initial_balances(vec![(delegator, amount)])
        .build();

    suite.bond(delegator, amount).unwrap();

    // User now has the same amount of LSD tokens as they staked
    let lsd = suite.query_lsd_token().unwrap();
    let lsd_token_balance = suite.query_cw20_balance(delegator, &lsd).unwrap();

    // Wait until next epoch and reinvest to trigger delegation
    suite.update_time(23 * HOUR);
    suite.reinvest().unwrap();

    suite.unbond(delegator, &lsd, lsd_token_balance).unwrap();

    // Unbonding gets triggered every 28 / 7 = 4 days = 24 hours * 4 = 96 hours,
    // so we need to wait 5 more epochs after last reinvest
    suite.update_time(5 * 23 * HOUR);
    suite.reinvest().unwrap();

    // After triggering unbonding, we should have a pending unbonding
    let supply = SUPPLY.query(&suite.app.wrap(), suite.hub.clone()).unwrap();
    let unbonding_amount = amount - suite.query_balance(suite.hub.as_str(), "FUN").unwrap();
    assert_eq!(supply.total_unbonding.u128(), unbonding_amount);

    // Simulate liveness slash (0.1%)
    suite.slash("testvaloper1", Decimal::permille(1)).unwrap();

    suite.update_time(5 * MINUTE);
    suite.check_slash().unwrap();

    // Make sure supply is updated
    let supply = SUPPLY.query(&suite.app.wrap(), suite.hub.clone()).unwrap();

    let storage = suite.read_hub_storage();
    let unbonding: Vec<_> = UNBONDING
        .range(&storage, None, None, Order::Ascending)
        .map(|ub| ub.unwrap())
        .collect();
    // allow a bit of rounding error
    assert_approx_eq!(
        unbonding[0].1[0].amount.u128(),
        unbonding_amount - unbonding_amount / 1000,
        "0.00006"
    );
    assert!(
        unbonding[0].1[0].amount.u128() <= unbonding_amount - unbonding_amount / 1000,
        "should be rounded down, if at all"
    );
    assert_eq!(
        supply.total_unbonding.u128(),
        unbonding[0].1[0].amount.u128()
    );
    assert_approx_eq!(
        supply.total_bonded,
        Uint128::new(amount - unbonding_amount).mul_floor(Decimal::permille(999)),
        "0.00009"
    );
    assert!(
        supply.total_bonded
            <= Uint128::new(amount - unbonding_amount).mul_floor(Decimal::permille(999)),
        "should be rounded down, if at all"
    );
    let bonded = BONDED.load(&storage).unwrap();
    assert_eq!(bonded[0], ("testvaloper1".to_string(), supply.total_bonded));
    let balance = suite.query_balance(suite.hub.as_str(), "FUN").unwrap();
    assert_eq!(
        supply.claims.u128(),
        supply.total_unbonding.u128() + balance
    );
    assert_approx_eq!(
        supply.claims.u128(),
        unbonding_amount - unbonding_amount / 1000 + balance,
        "0.00006"
    );

    // Wait until unbonding is complete
    suite.update_time(28 * DAY);
    suite.process_native_unbonding();

    // Claim delegator's tokens
    suite.claim(delegator).unwrap();
    let received = suite.query_balance(delegator, "FUN").unwrap();
    assert_approx_eq!(received, amount - amount / 1000, "0.00006");
    assert!(
        received <= amount - amount / 1000,
        "should be rounded down, if at all"
    );
}
