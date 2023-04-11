use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, BlockInfo, CustomQuery, Deps, StdResult, Storage, Timestamp, Uint128};
use cw_storage_plus::Map;

// copied and adapted from cw-controllers

#[cw_serde]
pub struct ClaimsResponse {
    pub claims: Vec<Claim>,
}

#[cw_serde]
pub struct Claim {
    pub amount: Uint128,
    pub release_at: Timestamp,
}

impl Claim {
    pub fn new(amount: u128, released: Timestamp) -> Self {
        Claim {
            amount: amount.into(),
            release_at: released,
        }
    }
}

pub struct Claims<'a>(Map<'a, &'a Addr, Vec<Claim>>);

impl<'a> Claims<'a> {
    pub const fn new(storage_key: &'a str) -> Self {
        Claims(Map::new(storage_key))
    }

    /// This creates a claim, such that the given address can claim an amount of tokens after
    /// the release date.
    pub fn create_claim(
        &self,
        storage: &mut dyn Storage,
        addr: &Addr,
        amount: Uint128,
        release_at: Timestamp,
    ) -> StdResult<()> {
        // add a claim to this user to get their tokens after the unbonding period
        self.0.update(storage, addr, |old| -> StdResult<_> {
            let mut claims = old.unwrap_or_default();
            claims.push(Claim { amount, release_at });
            Ok(claims)
        })?;
        Ok(())
    }

    /// This iterates over all mature claims for the address, and removes them, up to an optional cap.
    /// it removes the finished claims and returns the total amount of tokens to be released.
    pub fn claim_tokens(
        &self,
        storage: &mut dyn Storage,
        addr: &Addr,
        block: &BlockInfo,
        claim_amount: impl Fn(&Claim) -> Uint128,
        cap: Option<Uint128>,
    ) -> StdResult<Uint128> {
        let mut to_send = Uint128::zero();
        self.0.update(storage, addr, |claim| -> StdResult<_> {
            let (_send, waiting): (Vec<_>, _) =
                claim.unwrap_or_default().into_iter().partition(|c| {
                    // if mature and we can pay fully, then include in _send
                    if c.release_at <= block.time {
                        let c_amount = claim_amount(c);
                        if let Some(limit) = cap {
                            if to_send + c_amount > limit {
                                return false;
                            }
                        }
                        to_send += c_amount;
                        true
                    } else {
                        // not to send, leave in waiting and save again
                        false
                    }
                });
            Ok(waiting)
        })?;
        Ok(to_send)
    }

    pub fn query_claims<Q: CustomQuery>(
        &self,
        deps: Deps<Q>,
        address: &Addr,
    ) -> StdResult<ClaimsResponse> {
        let claims = self.0.may_load(deps.storage, address)?.unwrap_or_default();
        Ok(ClaimsResponse { claims })
    }
}
