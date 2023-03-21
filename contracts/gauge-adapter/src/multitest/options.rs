use cosmwasm_std::Addr;

use crate::multitest::suite::SuiteBuilder;

#[test]
fn option_queries() {
    let validators = vec![
        ("junovaloper1t8ehvswxjfn3ejzkjtntcyrqwvmvuknzmvtaaa", "1.0"),
        ("junovaloper196ax4vc0lwpxndu9dyhvca7jhxp70rmcqcnylw", "1.0"),
        ("junovaloper1y0us8xvsvfvqkk9c6nt5cfyu5au5tww2wsdcwk", "1.0"),
    ];
    let suite = SuiteBuilder::new()
        .with_chain_validators(validators.clone())
        .build();

    // get all options
    let options = suite.query_all_options().unwrap();
    assert_eq!(
        options,
        validators
            .iter()
            .map(|v| v.0.to_string())
            .collect::<Vec<_>>()
    );

    // check option validity
    for v in validators {
        assert!(suite.query_check_option(v.0.to_string()).unwrap());
    }
    assert!(!suite
        .query_check_option(Addr::unchecked("invalid").to_string())
        .unwrap());
}

#[test]
fn query_validators_with_commission_cap() {
    let validators = vec![
        ("junovaloper1t8ehvswxjfn3ejzkjtntcyrqwvmvuknzmvtaaa", "1.0"),
        ("junovaloper196ax4vc0lwpxndu9dyhvca7jhxp70rmcqcnylw", "0.2"),
        ("junovaloper196ax4vc0lwpxndu9dyhvca7jhxpsdawytdfsgf", "0.3"),
        ("junovaloper196ax4vc0lwpxndu9dyhvca7jhxp70rdadsadsa", "0.8"),
        ("junovaloper1y0us8xvsvfvqkk9c6nt5cfyu5au5tww2wsdcwk", "1.0"),
    ];
    let suite = SuiteBuilder::new()
        .with_chain_validators(validators.clone())
        .with_max_allowed_commission("0.3")
        .build();

    // get all options
    let options = suite.query_all_options().unwrap();
    assert_eq!(
        options,
        vec![
            "junovaloper196ax4vc0lwpxndu9dyhvca7jhxp70rmcqcnylw",
            "junovaloper196ax4vc0lwpxndu9dyhvca7jhxpsdawytdfsgf"
        ]
    );
}
