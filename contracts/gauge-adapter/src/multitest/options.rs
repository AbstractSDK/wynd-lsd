use cosmwasm_std::Addr;

use crate::multitest::suite::SuiteBuilder;

#[test]
fn option_queries() {
    let validators = vec![
        "junovaloper1t8ehvswxjfn3ejzkjtntcyrqwvmvuknzmvtaaa",
        "junovaloper196ax4vc0lwpxndu9dyhvca7jhxp70rmcqcnylw",
        "junovaloper1y0us8xvsvfvqkk9c6nt5cfyu5au5tww2wsdcwk",
    ];
    let suite = SuiteBuilder::new()
        .with_chain_validators(validators.clone())
        .build();

    // get all options
    let options = suite.query_all_options().unwrap();
    assert_eq!(
        options,
        validators.iter().map(|v| v.to_string()).collect::<Vec<_>>()
    );

    // check option validity
    for v in validators {
        assert!(suite.query_check_option(v.to_string()).unwrap());
    }
    assert!(!suite
        .query_check_option(Addr::unchecked("invalid").to_string())
        .unwrap());
}
