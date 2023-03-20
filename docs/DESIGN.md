# Project Design

This is a high-level document of the workflows from a user perspective.
We can discuss algorithms in more detailed architecture documents.

## Overview

WYND LSD is a staking derivative for the native staking asset on the same chain.
It assumes it can delegate/undelegate/redelegate/query delegations as atomic
operations in the same transaction. This is different from other models that
use ICA to invest in staking derivatives on other chains.

Basically, WYND would deploy LSD contracts on the chains where we wish to enable
staking derivatives, and it can compose nicely with any other DeFi apps on that chains.
WYND DAO (on Juno) would use IBC to send governance update messages to receive any funds
if it is deployed on a different chain, but these occur maybe 1/week or less, rather
than every transaction.

## Usage Example

Let's look at how wyJUNO (an instance of WYND LSD on Juno) would work for an end user.

To start, I `Bond` my $JUNO into the contract and receive a number of $wyJUNO.
Since each $wyJUNO is backed by more than 1 $JUNO (due to auto-compounding),
we may only get say 90 $wyJUNO when we deposit 100 $JUNO. However, the number is
such that if you withdraw right away, you get back the original $JUNO in full.

Now I hold 90 $wyJUNO. I can use this in various DeFi protocols. For example,
maybe I add it to the wyJUNO-WYND liquidity pool on WYND DEX. The entire time
I hold this, it is auto-compounding daily. That is, the value of wyJUNO is
constantly increasing.

After 6 months staked in the LP, I pull out and recover the $wyJUNO and $WYND into
my wallet. I now wish to `Unbond` my $wyJUNO to recover normal $JUNO. We check the
current exchange rate due to auto-compounding, and find those 90 $wyJUNO are now worth
120 $JUNO. When you `Unbond`, your 90 $wyJUNO are burnt and you get a claim for 120 $JUNO,
which will take the standard unbonding period (plus up to 24 hours). At that point,
you can `Withdraw` the $JUNO.

This allows you to have a similar experience to auto-compounding native staking
(similar rewards, similar unbonding), but with the ability to use your yield bearing
tokens in various DeFi protocols in the meantime.

## Workflow

### Initialization

We start with a weighted set of validators (address and percentage) to delegate to.
The total weights must sum to 100%, but they can be distributed in any way, no need
to be equally split. We also set a gov contract (WYND DAO) that can update the configuration.
The `exchange_rate` ($JUNO per $wyJUNO) is set at 1.0 to begin.

### Bonding (Deposit)

At any time, a user can send the native staking asset to the contract and receive
LSD tokens based on the current `exchange_rate`. The contract doesn't delegate
these immediately to make this a cheap operation.

### Unbond

Any time a holder of the LSD token can request the native asset locked by it.
At the time of Unbonding, the contract burns the wyJUNO and creates Claims
to match the JUNO backing those wyJUNO. The Claim date should be next reinvest epoch
plus the unbonding_period, when the tokens will have been withdrawn.

### Reinvest

This can be called once every epoch (eg. daily) to handle all Delegating/Undelegating.

Over the epoch, we have accumulated various staking tokens via Bonding. We have also
received a number of Unbonding requests. This is when we process them all. The workflow is:

1. Withdraw rewards from all delegations
2. Calculate gain/loss between `(balance + delegated + currently unstaking)` - `(reserves + open claims)`
3. If this is a "gain", then all excess balance will be delegated to the validators, split accoring to weights
4. If this is a "loss", then we trigger unbonding on the validators, proportional to weight
5. Update `exchange_rate` based on the current balances

I define `reserves = (LSD token suply * exchange_rate)`.
The number of tokens needed to uphold the promise of future payouts.

#### Exchange Rate Calculation

The assets and obligations of the LSD must always sum up.

Assets: `liquid balance (JUNO) + delegations` (note: delegations include currently unbonding stake)
Obligations: `LSD balance (wyJUNO) * exchange_rate + open_claims`

In absence of slashing, the `exchange_rate` should always be monotonically increasing.

`Deposit`: increase assets (liquid balance) at same level as obligaton (LSD * exchange_rate)

`Withdraw`: keep obligations equal (LSD * exchange_rate decreases, open_claims increase)

`Reinvest`: The withdrawal of rewards should increase liquid balance
   We take all tokens that were sent for delegation (bonded), combine them with rewards
  and "handle imbalance" by distributing them among validators according to weights.
  This will have the effect of increasing the exchange_rate to make assets match obligations.

TODO: examples

TODO: slashing

### Withdrawing

Once a Claim has matured, the user can withdraw the corresponding amount of native staking
asset and delete the Claim. The reinvest design above guarantees that there will be sufficient
liquid balance to cover any mature claims.

### Updating Validators

The validators can be updated by the governance contract, but they cannot be updated more often
than once per `unbonding_period` to safely handle rebonding.

When a valid update request comes in, we compare the current validator weights with
the new validator weights to determine which changes there are, and calculate
the minimum set of redelgations we must make in order to shift the existing delegations
between validators. Because we cannot re-redelegate given stake until a full unbonding_period
has passed, we perform this update at that rate.

When we exeute this, we first withdraw all existing rewards, then execute the redelegations.
When the next `Reinvest` trigger comes in, those rewards will be delegated according to the
new distributions

These updates will be managed by a gauge with an appropriate epoch time.

## Useful Queries

TODO: refine

* exchange_rate
* claims info
* current validator set
* reinvest timing (when can it be called, last call)
* validator update timing (when can it be called, last call)

## Open Questions

* How to handle slashing?
* How to limit unbondings? The LSD contract can only have a limited number of unbondings on a given validator in progress. Default 3, on Juno 7.
  * Reinvest should somehow spread out the unbonding so we can unbond every day and not overwhelm them.
  * First version can just unbond all equally when needed. Add better once all Rust tests pass.