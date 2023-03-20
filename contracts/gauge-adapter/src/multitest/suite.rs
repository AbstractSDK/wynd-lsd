use anyhow::Result as AnyResult;

use cosmwasm_std::{testing::mock_env, Addr, CosmosMsg, Decimal, Validator};
use cw20::{BalanceResponse, Cw20QueryMsg};
use cw_multi_test::{App, ContractWrapper, Executor};

use cw_placeholder::msg::InstantiateMsg as PlaceholderContractInstantiateMsg;
use wynd_lsd_hub::msg::{
    InstantiateMsg as HubInstantiateMsg, QueryMsg as HubQueryMsg, TokenInitInfo,
    ValidatorSetResponse,
};

use crate::msg::{
    AdapterQueryMsg, AllOptionsResponse, CheckOptionResponse, MigrateMsg, SampleGaugeMsgsResponse,
};

fn store_gauge_adapter(app: &mut App) -> u64 {
    let contract = Box::new(
        ContractWrapper::new_with_empty(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        )
        .with_migrate_empty(crate::contract::migrate),
    );

    app.store_code(contract)
}

fn store_hub(app: &mut App) -> u64 {
    let contract = Box::new(
        ContractWrapper::new_with_empty(
            wynd_lsd_hub::contract::execute,
            wynd_lsd_hub::contract::instantiate,
            wynd_lsd_hub::contract::query,
        )
        .with_reply_empty(wynd_lsd_hub::contract::reply),
    );

    app.store_code(contract)
}

fn store_cw20(app: &mut App) -> u64 {
    let contract = Box::new(ContractWrapper::new(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    ));

    app.store_code(contract)
}

fn store_placeholder_code(app: &mut App) -> u64 {
    let placeholder_contract = Box::new(ContractWrapper::new_with_empty(
        cw_placeholder::contract::execute,
        cw_placeholder::contract::instantiate,
        cw_placeholder::contract::query,
    ));

    app.store_code(placeholder_contract)
}

#[derive(Debug)]
pub struct SuiteBuilder {
    chain_validators: Vec<String>,
    validator_commission: Decimal,

    comission: Decimal,
    validators: Vec<(String, Decimal)>,
    epoch_period: u64,
    unbond_period: u64,
    max_concurrent_unbondings: u64,
    via_placeholder: bool,
}

impl SuiteBuilder {
    pub fn new() -> Self {
        Self {
            chain_validators: vec![
                "junovaloper1t8ehvswxjfn3ejzkjtntcyrqwvmvuknzmvtaaa".to_string(),
                "junovaloper196ax4vc0lwpxndu9dyhvca7jhxp70rmcqcnylw".to_string(),
                "junovaloper1y0us8xvsvfvqkk9c6nt5cfyu5au5tww2wsdcwk".to_string(),
            ],
            validator_commission: Decimal::percent(1),
            comission: Decimal::percent(10),
            validators: vec![(
                "junovaloper196ax4vc0lwpxndu9dyhvca7jhxp70rmcqcnylw".to_string(),
                Decimal::one(),
            )],
            epoch_period: 21 * 24 * 60 * 60,
            unbond_period: 21 * 24 * 60 * 60,
            max_concurrent_unbondings: 7,
            via_placeholder: false,
        }
    }

    pub fn with_chain_validators(mut self, chain_validators: Vec<&str>) -> Self {
        self.chain_validators = chain_validators
            .into_iter()
            .map(|v| v.to_string())
            .collect();
        self
    }

    #[allow(unused)]
    pub fn with_validator_commission(mut self, validator_commission: Decimal) -> Self {
        self.validator_commission = validator_commission;
        self
    }

    #[allow(unused)]
    pub fn with_max_concurrent_unbondings(mut self, max_concurrent_unbondings: u64) -> Self {
        self.max_concurrent_unbondings = max_concurrent_unbondings;
        self
    }

    #[allow(unused)]
    pub fn with_comission(mut self, comission: Decimal) -> Self {
        self.comission = comission;
        self
    }

    #[allow(unused)]
    pub fn with_initial_validators(mut self, validators: Vec<(&str, Decimal)>) -> Self {
        self.validators = validators
            .into_iter()
            .map(|(v, c)| (v.to_string(), c))
            .collect();
        self
    }

    #[allow(unused)]
    pub fn with_epoch_period(mut self, epoch_period: u64) -> Self {
        self.epoch_period = epoch_period;
        self
    }

    #[allow(unused)]
    pub fn with_unbond_period(mut self, unbond_period: u64) -> Self {
        self.unbond_period = unbond_period;
        self
    }

    #[allow(unused)]
    pub fn via_placeholder(mut self) -> Self {
        self.via_placeholder = true;
        self
    }

    #[track_caller]
    pub fn build(self) -> Suite {
        let mut app = App::default();
        let owner = Addr::unchecked("owner");

        let factory_code_id = store_hub(&mut app);
        let cw20_code_id = store_cw20(&mut app);
        let gauge_adapter_code_id = store_gauge_adapter(&mut app);
        let place_holder_id = store_placeholder_code(&mut app);

        let epoch_length = 86_400;

        let hub = app
            .instantiate_contract(
                factory_code_id,
                owner.clone(),
                &HubInstantiateMsg {
                    treasury: "treasury".to_string(),
                    owner: owner.to_string(),
                    comission: self.comission,
                    validators: self.validators,
                    cw20_init: TokenInitInfo {
                        cw20_code_id,
                        label: "wyJUNO".to_string(),
                        name: "Staked Juno".to_string(),
                        symbol: "WYJUNO".to_string(),
                        decimals: 6,
                        initial_balances: vec![],
                        marketing: None,
                    },
                    epoch_period: self.epoch_period,
                    unbond_period: self.unbond_period,
                    max_concurrent_unbondings: self.max_concurrent_unbondings,
                    liquidity_discount: Decimal::percent(4),
                },
                &[],
                "Wyndex LSD Hub",
                Some(owner.to_string()),
            )
            .unwrap();

        let adapter_init_msg = crate::msg::InstantiateMsg {
            hub: hub.to_string(),
        };
        let adapter_label = "Gauge Adapter";

        let gauge_adapter = if !self.via_placeholder {
            app.instantiate_contract(
                gauge_adapter_code_id,
                owner.clone(),
                &adapter_init_msg,
                &[],
                adapter_label,
                Some(owner.to_string()),
            )
            .unwrap()
        } else {
            // start with placeholder
            let contract_addr = app
                .instantiate_contract(
                    place_holder_id,
                    owner.clone(),
                    &PlaceholderContractInstantiateMsg {},
                    &[],
                    adapter_label,
                    Some(owner.to_string()),
                )
                .unwrap();
            // now migrate to real one
            app.migrate_contract(
                owner.clone(),
                contract_addr.clone(),
                &MigrateMsg::Init(adapter_init_msg),
                gauge_adapter_code_id,
            )
            .unwrap();
            contract_addr
        };

        app.init_modules(|router, api, storage| -> AnyResult<()> {
            for val in self.chain_validators {
                router.staking.add_validator(
                    api,
                    storage,
                    &mock_env().block,
                    Validator {
                        address: val,
                        commission: self.validator_commission,
                        max_commission: Decimal::one(),
                        max_change_rate: Decimal::one(),
                    },
                )?;
            }
            Ok(())
        })
        .unwrap();

        Suite {
            owner,
            app,
            hub,
            gauge_adapter,
            epoch_length,
        }
    }
}

pub struct Suite {
    pub owner: Addr,
    pub app: App,
    pub hub: Addr,
    pub gauge_adapter: Addr,
    pub epoch_length: u64,
}

impl Suite {
    #[allow(unused)]
    pub fn next_block(&mut self, time: u64) {
        self.app.update_block(|block| {
            block.time = block.time.plus_seconds(time);
            block.height += 1
        });
    }

    #[allow(unused)]
    pub fn sample_gauge_msgs(&self, selected: Vec<(String, Decimal)>) -> Vec<CosmosMsg> {
        let msgs: SampleGaugeMsgsResponse = self
            .app
            .wrap()
            .query_wasm_smart(
                self.gauge_adapter.clone(),
                &AdapterQueryMsg::SampleGaugeMsgs { selected },
            )
            .unwrap();
        msgs.execute
    }

    #[allow(unused)]
    pub fn query_cw20_balance(&self, user: &str, contract: &Addr) -> AnyResult<u128> {
        let balance: BalanceResponse = self.app.wrap().query_wasm_smart(
            contract,
            &Cw20QueryMsg::Balance {
                address: user.to_owned(),
            },
        )?;
        Ok(balance.balance.into())
    }

    #[allow(unused)]
    pub fn query_validator_set(&self) -> AnyResult<Vec<(String, Decimal)>> {
        let res: ValidatorSetResponse = self
            .app
            .wrap()
            .query_wasm_smart(self.hub.clone(), &HubQueryMsg::ValidatorSet {})?;
        Ok(res.validator_set)
    }

    pub fn query_all_options(&self) -> AnyResult<Vec<String>> {
        let res: AllOptionsResponse = self
            .app
            .wrap()
            .query_wasm_smart(self.gauge_adapter.clone(), &AdapterQueryMsg::AllOptions {})?;

        Ok(res.options)
    }

    pub fn query_check_option(&self, option: String) -> AnyResult<bool> {
        let res: CheckOptionResponse = self.app.wrap().query_wasm_smart(
            self.gauge_adapter.clone(),
            &AdapterQueryMsg::CheckOption { option },
        )?;

        Ok(res.valid)
    }
}
