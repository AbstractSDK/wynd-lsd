use crate::ContractError;

use super::suite::SuiteBuilder;

use cosmwasm_std::{Decimal, Uint128};

const DAY: u64 = 24 * HOUR;
const HOUR: u64 = 60 * 60;

#[test]
fn bond_claim_without_unbond() {
    let delegators = &["delegator1", "delegator2"];

    let initial_staking_amount = 1_000_000u128;
    let mut suite = SuiteBuilder::new()
        .with_initial_balances(vec![
            (delegators[0], 2_000_000u128),
            (delegators[1], 2_000_000u128),
        ])
        .build();

    // Simulate deposits for some of the users of all their stake
    suite.bond(delegators[0], initial_staking_amount).unwrap();
    suite.bond(delegators[1], initial_staking_amount).unwrap();

    let lsd = suite.query_lsd_token().unwrap();

    // Users now have the same amount of LSD tokens as they staked
    let lsd_token_balance = suite.query_cw20_balance(delegators[0], &lsd).unwrap();
    assert_eq!(
        lsd_token_balance,
        Uint128::new(initial_staking_amount).u128()
    );

    // Submit a claim before doing unbonding, it should raise a NothingToClaim Error
    assert!(matches!(
        suite.claim(delegators[0]).unwrap_err().downcast().unwrap(),
        ContractError::NothingToClaim {}
    ));
}

#[test]
fn bond_with_native_unbond_with_wrong_token() {
    let delegator = "delegator";

    let amount = 1_000_003u128;
    let mut suite = SuiteBuilder::new()
        .with_initial_balances(vec![(delegator, amount)])
        .with_validators(vec![
            ("testvaloper1", Decimal::percent(50)),
            ("testvaloper2", Decimal::percent(50)),
        ])
        .with_periods(23 * HOUR, 28 * DAY)
        .build();

    // deposit some tokens to stake
    suite.bond(delegator, amount).unwrap();

    // wait until next epoch and do first reinvest to actually delegate
    suite.update_time(23 * HOUR);
    suite.reinvest().unwrap();

    // wait one more epoch to get rewards
    suite.update_time(23 * HOUR);
    suite.reinvest().unwrap();
    // now unbond but with an address that is not the lsd token

    // Expect InvalidToken
    assert!(matches!(
        suite
            .unbond(delegator, &suite.other_token_contract.clone(), amount)
            .unwrap_err()
            .downcast()
            .unwrap(),
        ContractError::InvalidToken {}
    ));
}
