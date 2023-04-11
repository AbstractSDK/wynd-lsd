use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::Item;

#[cw_serde]
pub struct Config {
    /// Address of the lsd-hub contract
    pub hub: Addr,
    /// Address of the staking swap pool to trade on
    pub pair: Addr,
    /// This is the denomination we can stake (and only one we accept for payments)
    pub bond_denom: String,
    /// This is the lsd token the users wishes to receive
    pub lsd_token: Addr,
}

pub const CONFIG: Item<Config> = Item::new("config");

/// Stack to push/pop on replies
pub const REPLY_INFO: Item<Addr> = Item::new("reply");
