use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{CosmosMsg, Decimal};

#[cw_serde]
pub struct InstantiateMsg {
    /// Address of the hub contract
    pub hub: String,
    /// Maximum allowed commision by validator to be included in voting set
    pub max_commission: Decimal,
}

#[cw_serde]
pub enum MigrateMsg {
    /// Used to instantiate from cw-placeholder
    Init(InstantiateMsg),
    /// Migrates from version <= v1.2.0
    Update { max_commission: Decimal },
}

// Queries copied from gauge-orchestrator for now (we could use a common crate for this)
/// Queries the gauge requires from the adapter contract in order to function
#[cw_serde]
#[derive(QueryResponses)]
pub enum AdapterQueryMsg {
    #[returns(crate::state::Config)]
    Config {},
    #[returns(AllOptionsResponse)]
    AllOptions {},
    #[returns(CheckOptionResponse)]
    CheckOption { option: String },
    #[returns(SampleGaugeMsgsResponse)]
    SampleGaugeMsgs {
        /// option along with weight
        /// sum of all weights should be 1.0 (within rounding error)
        selected: Vec<(String, Decimal)>,
    },
}

#[cw_serde]
pub struct AllOptionsResponse {
    pub options: Vec<String>,
}

#[cw_serde]
pub struct CheckOptionResponse {
    pub valid: bool,
}

#[cw_serde]
pub struct SampleGaugeMsgsResponse {
    pub execute: Vec<CosmosMsg>,
}
