use cosmwasm_schema::write_api;

use bond_router::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use cosmwasm_std::Empty;

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg,
        query: QueryMsg,
        migrate: Empty,
    }
}
