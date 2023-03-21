use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal};
use cw_storage_plus::Item;

#[cw_serde]
pub struct Config {
    /// Address of the hub contract
    pub hub: Addr,
    /// Maximum allowed commision by validator to be included in voting set
    pub max_commission: Decimal,
}

pub const CONFIG: Item<Config> = Item::new("config");
