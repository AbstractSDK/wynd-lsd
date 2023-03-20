use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::Item;

#[cw_serde]
pub struct Config {
    /// Address of the hub contract
    pub hub: Addr,
}

pub const CONFIG: Item<Config> = Item::new("config");
