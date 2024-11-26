use crate::{
    claim::{Claim, ClaimsResponse},
    msg::{
        ConfigResponse, ExchangeRateResponse, ExecuteMsg, InstantiateMsg, QueryMsg, ReceiveMsg,
        SupplyResponse, TargetValueResponse, TokenInitInfo, ValidatorSetResponse,
    },
};
use anyhow::Result as AnyResult;
use cosmwasm_std::{
    coins, to_json_binary, Addr, Coin, ContractInfoResponse, Decimal, Delegation, Empty,
    FullDelegation, MemoryStorage, StdResult, Storage, Uint128, Validator,
};
use cw20::{BalanceResponse, Cw20Coin, Cw20QueryMsg};
use cw20_base::msg::InstantiateMsg as Cw20InstantiateMsg;
use cw_multi_test::{
    App, AppResponse, Contract, ContractWrapper, Executor, StakingInfo, StakingSudo, SudoMsg,
};

fn contract_hub() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    )
    .with_reply(crate::contract::reply);

    Box::new(contract)
}

fn store_token_code() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    );
    Box::new(contract)
}

pub struct SuiteBuilder {
    pub token_name: String,
    pub token_symbol: String,
    pub token_decimals: u8,
    pub initial_balances: Vec<Cw20Coin>,
    pub validators: Vec<(String, Decimal)>,
    pub registered_validators: Vec<String>,
    pub validator_commission: Decimal,
    pub treasury_commission: Decimal,
    pub bounty_amount: Uint128,
    pub epoch_period: u64,
    pub unbond_period: u64,
    pub liquidity_discount: Decimal,
}

const DAY: u64 = 24 * HOUR;
const HOUR: u64 = 60 * 60;

impl SuiteBuilder {
    pub fn new() -> Self {
        Self {
            token_name: "Fun".to_owned(),
            token_symbol: "FUN".to_owned(),
            token_decimals: 9,
            initial_balances: vec![],
            validators: vec![("testvaloper1".to_string(), Decimal::percent(100))],
            registered_validators: vec![],
            validator_commission: Decimal::percent(5),
            treasury_commission: Decimal::percent(5),
            bounty_amount: Uint128::new(5),
            epoch_period: 23 * HOUR,
            unbond_period: 28 * DAY,
            liquidity_discount: Decimal::percent(4),
        }
    }

    pub fn with_initial_balances(mut self, balances: Vec<(&str, u128)>) -> Self {
        let initial_balances = balances
            .into_iter()
            .map(|(address, amount)| Cw20Coin {
                address: address.to_owned(),
                amount: amount.into(),
            })
            .collect::<Vec<_>>();
        self.initial_balances = initial_balances;
        self
    }

    pub fn with_liquidity_discount(mut self, discount: Decimal) -> Self {
        self.liquidity_discount = discount;
        self
    }

    pub fn with_validators(mut self, validators: Vec<(&str, Decimal)>) -> Self {
        let validators = validators
            .into_iter()
            .map(|(address, commission)| (address.to_owned(), commission))
            .collect::<Vec<_>>();
        self.validators = validators;
        self
    }

    pub fn with_registered_validators(mut self, validators: Vec<String>) -> Self {
        self.registered_validators = validators;
        self
    }

    pub fn with_periods(mut self, epoch_peroid: u64, unbond_period: u64) -> Self {
        self.epoch_period = epoch_peroid;
        self.unbond_period = unbond_period;
        self
    }

    #[track_caller]
    pub fn build(self) -> Suite {
        let mut app: App = App::default();
        let admin = Addr::unchecked("admin");
        // add validators
        let valopers = self.validators.iter().map(|(validator, _)| {
            Validator::new(
                validator.clone(),
                self.validator_commission,
                Decimal::percent(100),
                Decimal::percent(1),
            )
        });
        let valopers_registered = self.registered_validators.iter().map(|validator| {
            Validator::new(
                validator.clone(),
                self.validator_commission,
                Decimal::percent(100),
                Decimal::percent(1),
            )
        });

        let staking_info = StakingInfo {
            bonded_denom: "FUN".to_string(),
            unbonding_time: self.unbond_period,
            apr: Decimal::percent(80),
        };
        let block_info = app.block_info();
        // Use init_modules to setup the validators
        app.init_modules(|router, api, storage| -> AnyResult<()> {
            router.staking.setup(storage, staking_info).unwrap();

            for valoper in valopers {
                router
                    .staking
                    .add_validator(api, storage, &block_info, valoper)
                    .unwrap();
            }
            for valoper in valopers_registered {
                router
                    .staking
                    .add_validator(api, storage, &block_info, valoper)
                    .unwrap();
            }

            self.initial_balances.clone().into_iter().for_each(|coin| {
                router
                    .bank
                    .init_balance(
                        storage,
                        &Addr::unchecked(coin.address),
                        vec![Coin {
                            amount: coin.amount,
                            denom: "FUN".to_string(),
                        }],
                    )
                    .unwrap()
            });

            Ok(())
        })
        .unwrap();

        let hub_id = app.store_code(contract_hub());
        let cw20_id = app.store_code(store_token_code());
        let hub = app
            .instantiate_contract(
                hub_id,
                admin.clone(),
                &InstantiateMsg {
                    treasury: "treasury".to_string(),
                    commission: self.treasury_commission,
                    validators: self.validators,
                    owner: "owner".to_string(),

                    epoch_period: self.epoch_period,
                    unbond_period: self.unbond_period,
                    max_concurrent_unbondings: 7,
                    cw20_init: TokenInitInfo {
                        label: "label".to_string(),
                        cw20_code_id: cw20_id,
                        name: "funLSD".to_string(),
                        symbol: "fLSD".to_string(),
                        decimals: 6,
                        initial_balances: vec![],
                        marketing: None,
                    },
                    liquidity_discount: self.liquidity_discount,
                    tombstone_treshold: Decimal::percent(3),
                    slashing_safety_margin: 10 * 60,
                },
                &[],
                "hub",
                Some(admin.to_string()),
            )
            .unwrap();

        let other_token_contract = app
            .instantiate_contract(
                cw20_id,
                admin,
                &Cw20InstantiateMsg {
                    name: "other".to_owned(),
                    symbol: "OTHER".to_owned(),
                    decimals: 9,
                    initial_balances: self.initial_balances,
                    mint: None,
                    marketing: None,
                },
                &[],
                "vesting",
                None,
            )
            .unwrap();

        Suite {
            app,
            hub,
            other_token_contract,
        }
    }
}

pub struct Suite {
    pub app: App,
    pub hub: Addr,
    pub other_token_contract: Addr,
}

impl Suite {
    /// update block's time to simulate passage of time
    pub fn update_time(&mut self, time_update: u64) {
        self.app
            .update_block(|block: &mut cosmwasm_std::BlockInfo| {
                block.time = block.time.plus_seconds(time_update);
                block.height += time_update / 5;
            })
    }

    pub fn reinvest(&mut self) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked("anyone"),
            self.hub.clone(),
            &ExecuteMsg::Reinvest {},
            &[],
        )
    }

    pub fn update_liquidity_discount(
        &mut self,
        sender: &str,
        new_discount: Decimal,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.hub.clone(),
            &ExecuteMsg::UpdateLiquidityDiscount { new_discount },
            &[],
        )
    }

    pub fn bond(&mut self, sender: &str, amount: u128) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.hub.clone(),
            &ExecuteMsg::Bond {},
            &coins(amount, "FUN"),
        )
    }

    pub fn unbond(
        &mut self,
        sender: &str,
        token_contract: &Addr,
        balance: u128,
    ) -> AnyResult<AppResponse> {
        let msg = to_json_binary(&ReceiveMsg::Unbond {})?;

        self.app.execute_contract(
            Addr::unchecked(sender),
            token_contract.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: self.hub.clone().to_string(),
                amount: balance.into(),
                msg,
            },
            &[],
        )
    }

    pub fn check_slash(&mut self) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked("anyone"),
            self.hub.clone(),
            &ExecuteMsg::CheckSlash {},
            &[],
        )
    }

    pub fn claim(&mut self, sender: &str) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.hub.clone(),
            &ExecuteMsg::Claim {},
            &[],
        )
    }

    /// returns address' balance of native token
    pub fn set_validators(
        &mut self,
        sender: &str,
        new_validators: Vec<(String, Decimal)>,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.hub.clone(),
            &ExecuteMsg::SetValidators { new_validators },
            &[],
        )
    }
    pub fn query_balance(&self, user: &str, denom: &str) -> AnyResult<u128> {
        Ok(self.app.wrap().query_balance(user, denom)?.amount.u128())
    }

    /// Queries all delegations of the hub contract
    pub fn query_delegations(&self) -> AnyResult<Vec<Delegation>> {
        Ok(self.app.wrap().query_all_delegations(&self.hub)?)
    }

    /// Queries all full delegations of the hub contract
    pub fn query_full_delegations(&self) -> AnyResult<Vec<FullDelegation>> {
        let s = self
            .query_validator_set()?
            .into_iter()
            .map(|(val, _)| {
                self.app
                    .wrap()
                    .query_delegation(&self.hub, &Addr::unchecked(val))
            })
            .collect::<StdResult<Vec<_>>>()?;
        Ok(s.into_iter().flatten().collect())
    }

    pub fn query_cw20_balance(&self, user: &str, contract: &Addr) -> AnyResult<u128> {
        let balance: BalanceResponse = self.app.wrap().query_wasm_smart(
            contract,
            &Cw20QueryMsg::Balance {
                address: user.to_owned(),
            },
        )?;
        Ok(balance.balance.into())
    }

    pub fn query_contract_admin(&self, contract: &Addr) -> AnyResult<String> {
        let contract_info: ContractInfoResponse =
            self.app.wrap().query_wasm_contract_info(contract)?;
        Ok(contract_info.admin.map(Into::into).unwrap_or_default())
    }

    pub fn query_exchange_rate(&self) -> AnyResult<Decimal> {
        let resp: ExchangeRateResponse = self
            .app
            .wrap()
            .query_wasm_smart(self.hub.clone(), &QueryMsg::ExchangeRate {})?;
        Ok(resp.exchange_rate)
    }

    pub fn query_tvl(&self) -> AnyResult<Uint128> {
        Ok(self
            .app
            .wrap()
            .query_wasm_smart::<SupplyResponse>(self.hub.clone(), &QueryMsg::Supply {})?
            .supply
            .total_bonded)
    }

    pub fn query_target_value(&self) -> AnyResult<Decimal> {
        let resp: TargetValueResponse = self
            .app
            .wrap()
            .query_wasm_smart(self.hub.clone(), &QueryMsg::TargetValue {})?;
        Ok(resp.target_value)
    }

    pub fn query_lsd_token(&self) -> AnyResult<Addr> {
        let balance: ConfigResponse = self
            .app
            .wrap()
            .query_wasm_smart(self.hub.clone(), &QueryMsg::Config {})?;
        Ok(balance.token_contract)
    }

    pub fn query_claims(&self, claim_addr: String) -> AnyResult<Vec<Claim>> {
        let claims: ClaimsResponse = self.app.wrap().query_wasm_smart(
            self.hub.clone(),
            &QueryMsg::Claims {
                address: claim_addr,
            },
        )?;
        Ok(claims.claims)
    }

    /// Processes the native unbonding queue
    /// This is done while updating the block
    pub fn process_native_unbonding(&mut self) {}

    pub fn query_validator_set(&self) -> AnyResult<Vec<(String, Decimal)>> {
        let vals: ValidatorSetResponse = self
            .app
            .wrap()
            .query_wasm_smart(self.hub.clone(), &QueryMsg::ValidatorSet {})?;
        Ok(vals.validator_set)
    }

    /// This let's us use lower level query type functions on a synthetic copy of the state of the hub contract storage
    pub fn read_hub_storage(&self) -> MemoryStorage {
        let mut storage = MemoryStorage::new();
        for (key, value) in self.app.dump_wasm_raw(&self.hub) {
            storage.set(&key, &value);
        }
        storage
    }

    pub fn slash(&mut self, validator: &str, amount: Decimal) -> AnyResult<AppResponse> {
        self.app.sudo(
            StakingSudo::Slash {
                validator: validator.to_string(),
                percentage: amount,
            }
            .into(),
        )
    }
}
