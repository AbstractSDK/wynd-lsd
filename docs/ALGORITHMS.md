# Algorithms

This explains some of the code math we need to maintain.

## Definitions

`native token` - The native staking token of the chain. eg. `ujuno`, `uosmo`, etc

`lsd token` - The cw20 tokens issued by `lsd-hub` to represent share of ownership of the staked assets

`hub balance` - Number of `native tokens` held by the `lsd-hub`

`lsd supply` - Total supply of `lsd tokens`

`bonded tokens` - Number of `native tokens` delegated by the `lsd-hub`, which are not in a state of unbonding

`unbonding tokens` - Number of `native tokens` delegated by the `lsd-hub`, which are currently unbonding

`claims` - Number of `native tokens` promised to ex-`lsd token`-holders who are in the process of withdrawing.
This generally corresponds to some amount of unbonding tokens.

`exchange_rate` - Number of `native tokens` per `lsd token`, used in deposit and withdraw

## Exchange Rate Invariant

The exchange rate should match the amount of AUM (Assets under Management) of the `lsd-hub`
divided by the `lsd supply`. This is a fair value of the assets one `lsd token` is worth.

```
AUM - Claims = lsd_supply * exchange_rate

hub_balance + bonded_tokens + unbonding_tokens - claims = lsd_tokens * exchange_rate
```

### Deposits

When depositing `X` "native tokens", we mint `X / exchange_rate` "lsd tokens".
Both sides increase by the same amount

### Withdraws

When withdrawing `X` "lsd tokens", we burn those tokens and create a new claim for 
`X * exchange_rate`. Both sides decrease by the same amount.

### Claims

When a claim is mature, a user can claim `X` "native tokens", removing their outstanding claim
and receiving those `X` tokens from hub_balance. The changes to claims and hub_balance are
equal and keeps the left side without change.

### Reinvest

This is the only time when exchange_rate is updated.

We first "Withdraw Rewards", which increases the hub_balance, without affecting any other
variable. To balance this out, we need to increase the `exchange_rate` to keep both sides
balance. This is the only place we need to recalculate `exchange_rate` and we can store
it in the state, just updating on Reinvest

#### Delegations

After withdrawing rewards, we need to delegate or undelegate tokens to keep all assets
not needed for claims to be productive (interest bearing). First we figure out how many
tokens we have that are not accounted for.

```
free_tokens = hub_balance + unbonding_tokens - claims
if free_tokens > 0 {
  delegate(free_tokens)
  hub_balance -= free_tokens
  bonded_tokens += free_tokens
} else if free_tokens < 0 {
  undelegate(-1 * free_tokens)
  bonded_tokens -= free_tokens
  unbonding_tokens += free_tokens
}
```

Check the invariant above and assure yourself that both branches maintain the invariant.

## Commission

This service is not provides for free, and the contract creator extracts a commission for the work.
This is taken when withdrawing tokens during reinvest. We must calculate the number of withdrawn tokens
(balance after withdraw - balance before withdraw). Of those, we take a percentage e.g. 5-10% and
send those to the collector address set in instantiate.

This commission is taken on the withdrawn rewards before any other calculation (like `free_tokens`)
and can be considered part of the withdrawing process. Effectively providing a slightly reduced
APR for the stakers in return for auto-compounding and fungibility of their assets.

## Time Periods

The Cosmos SDK staking module provides some limits we must adapt to.

* We can delegate tokens as often as we wish
* Undelegating tokens provides a delay of `unbonding_period` until these tokens appear in `hub_balance`
* Any given (delegator, validator) pair may only have N open unbondings at once (N typically 7)
* Any given (delegator, validator) may only have 1 open rebonding at once

We adjust to these by providing the following limits:

* Rebonding only once per `unbonding_period`. We must wait for one entire `unbonding_period` to have passed since the last `rebonding` before calling it again.
  This provides a limit on how often we can change the active validator set (`set_validators`)
* We allow delegating every reinvest period that has positive free_tokens. This can be any number (1 hour, 1 day, etc)
* We only allow unbonding once every `unbonding_period / N` days. We need to check this inside reinvest as a separate check for that case

### Detecting Unbonding

Much of the code assumes we have a local tracking of the state - which validators we
have delegated, how much they have bonded, and how much is unbonding. In order to do so,
we also need to detect when the unbonding finishes and those tokens have arrived in balance.

One issue with tracking [unbonding delegations](https://docs.cosmos.network/v0.46/modules/staking/02_state_transitions.html)
is that it occurs in EndBlock. This means that if we track these locally and say the unbonding will
complete at 13:00:00 Monday March 12th, then we process a block at 13:00:03 Monday March 12th,
the time has past, but it is likely that the unbonding has not yet been processed
(this is the first block since the unbonding finished, and the contracts are executed
before the unbonding is distributed).

The only time we really need to detect which tokens are bonded/unbonded/hub balance is in
`Reinvest{}`, to determine if those tokens in our balance are unbonded tokens for claims,
or newly arrived tokens we can delegate. In order to resolve this issue that time-based
detection will be off the first block after an unbonding occurs, is that we design the epochs
such that `Reinvest{}` will never be called the first block after an unbonding.

Nothing ever falls exactly to schedule, but if we design it such that `Reinvest{}` will never
be called closer than 1 hour to any unbonding completion, then we can ignore this case, and
trust the time-based local calculation of the tokens still unbonding.

### Epoch Selection

Given the above requirements, let's determine how this works. Unbonding can be triggered
by any previous `Reinvest` call within the last unbonding-period (UP, measured in hours).
The unbonding period is normally 2, 3, or 4 weeks. Let's look at how this works.

Every R hours, Reinvest is called (with a few minute margin of error).
If there exists any N, such that `|R * N - UP| < 0.5`, then it is possible that there is
an overlap. Assuming we only measure in complete hours, we can simplify this to 
`R * N = UP`. 

For the typical case, of 4 week UP and 1 day R, we see that N=28 is a perfect solution.
Clearly, we need to stagger them. Given 4 week (672 hour) UP, look at some possible R:

approx 3x/day
```
R = 7  => `672 / 7 = 96` xxx doesn't work
R = 9  => `672 / 9 = 74.666` WORKS!
```

approx 2x/day
```
R = 11  => `672 / 11 = 61.09` WORKS! (but close)
R = 13  => `672 / 13 = 51.69` WORKS!
```

approx 1x/day
```
R = 23  => `672 / 23 = 29.217` WORKS!
R = 25  => `672 / 25 = 26.88` WORKS!
```

**From these calculations, I would recommend 13, 23, or 25 hour intervals for a 4 week
unbonding period.**

You can revisit these calculations if deploying on a chain with different unbonding period.


## Slashing

We have shown that with proper selection of epoch parameters, we can track delegations,
unbonding, and rewards without ever querying the delegator states. This is much more
efficient than querying the chain for each validator. However, there is one thing
we cannot calculate locally, which is slashing events.

There are two types of slashing in the Cosmos SDK.
* Liveness (missed blocks) - 0.1% (or less) slash and short jail time (becomes inactive)
* Double Signing - 5% (or more) slash and permanently jailed

Once a validator is slashed for double signing, we never want to delegate to them again
and should remove them from the gauge as well. But minimally we blacklist them locally.
Delegating to a validator after a liveness fault is just a bit of lost funds, but we can
let the gauge manage that.

Note that some delegations will have been redelegated and can be slashed even if the
current validator is not misbehaving. To detect a double-slash / tombstone, we will
use some slashing threshold (add to config - eg 3% on a 5% slash penalty), and check
if they are inactive. A liveness fault may be inactive, but slashed far less than 3%.
Slashing on some redelegated shares could pass 3% but quite unlikely, but
the validator should not be marked inactive.

The fact of redelgations means that we have to check for slashing over all validators,
detect which ones were reduced (and update their local values), and figure out which,
if any, were tombstoned.


### Detection

In order to handle reacting to Slashing, I propose placing the slash detection off-chain
and just verifying it and updating on chains. For this, we can add a `CheckSlash{}`
entrypoint that can be called permissionlessly and verify if a slash happened.
Our contracts can assume that someone will call `CheckSlash` within a
reasonable amount of time (a few hours) after a slashing event. Or we could make this
a cron job, just like Reinvest.

If called, we first ensure the timing is proper. If the next pending unbonding is within say
10 minutes, we can fail as we may not be able to differentiate between unbonding execution
and slashing. Otherwise, we update the supply to account for any unbondings that have
matured and do the queries to compare actual state with out expectations (as defined above).

We query the current delegations on all validators, and for each of them, we compare the
amount delegated to the amount we believe to be on the validator - bonded + currently unbonding. 
Both bonded and unbonding tokens will be slashed, so we can just
compare `1 - (D' / D)` to determine the slashing penalty (D being our local track of
delegation, bonded and unbonding, on this validator, D' being the actual delegation reported).
If the amount is more than 0.001% below our expectations (allow some rounding),
we consider this validator to have been slashed. If the amount is greater than eg 3% (set
threshold in config) and validator is no longer active, we consider them tombstoned.

In the straightforward approach, this requires tracking the actual bonded and unbonding for every
single validator. Especially if the validator weights have changed, the amount unbonding will not
be proportional to their current weight. This requires adding more information to the store,
but we can use this storage-heavy approach until a more clever algorithm is developed in the future

### Adapting to Slashing

One we have detected a slashing event on a number of validators, we need to figure how much
was slashed to update our acounting. In the case of double-signing and tombstoning, we
withdraw all rewards from the malicious validator and block it from all future delegations.
For other cases, we don't provide in-protocol punishments, but let the gauges handle it. 
The cost is shared among all LSD holders.

To properly distribute the losses due to slashing, we want to determine how much was
lost from bonded tokens and how much from unbonding tokens. After that, we can adjust
pending claims, and reduce the `exchange_rate` so everything adds up again.

#### Bonded vs Unbonding slashes

Once we have detected some validators (V0...Vn) have been slashed, how do we adjust?

For each of these validators, we need to calculate the amount of *bonded* and *unbonded* slashed.
This assumes, we can track these numbers accurately for each validator, not just the pool
as a whole (and requires a unbonding queue *per validator*). Given that we can do:

`Di` - Expected delegations for validator i

`Di'` - Actual delegations for validator i

`Bi` - Expected Bonded tokens for validator i

`Ui` - Unbonded tokens for validator i

`SB` - Slashed bonded tokens

`SU` - Slashed unbonding tokens

We have stored `Di`, `Bi`, `Ui`, and can query `Di'` from the chain (in slash detection).

```
Ri = Di' / Di
Bi' = Bi * Ri
Ui' = Ui * Ri

SB = Sum(Bi - Bi': 0..n)
SU = Sum(Ui - Ui': 0..n)
```

With these number, we update each validators local records. Now, we calculate the global changes:

`RB` - Global multiplier for bonding tokens (eg a 3% slash would be 0.97)

`RU` - Global multiplier for unbonding tokens

```
RB = (B - SB) / B  
RU = (U - SU) / U  
```

With these numbers, we now update the global delegation stats, reducing the global bonded amount `B` by `SB`,
and for each pending unbonding in the list, multiply the amount by `RU`.

#### Updating Claims

Now that we have updated the delegation picture to the reality, we need to update outstanding claims,
as they are slashable for until the unbonding period completes (essential for security). We want
to multiply all currently pending claims (that haven't yet matured) by `RU`. This would take
an unbounded cost, as there may be many claims, so we work on a lazy version of this:

* Create a slashing event, `(start, end, multiplier)` equal to `(detection time, detection time + unbonding period, RU)`.
* Add this event to a queue (or better structure in later refactor)
* Every time someone claims their tokens, check of all slashing events `E`, such that `start < maturity`, `end > maturity`, and then multiply the claim by all slashes (C' = C * E0 * E1 ...)

Obviously we also need to update total claims amount properly. This is not `C' = C * RU`
(some can be mature but not yet claimed), but `C' = C - SU` (we split the slashed unbonding over
currently unbonding claims) 

#### Optimization

If the slash is very small, say < 0.02%, we may want to just absorb this in the bonded part and
not slash the unbonding queues or claims at all, which are more complicated. This makes
small liveness faults cheap to execute, while double signing is heavy and properly accounted for.

This would happen above when calculating `Bi'` and `Ui'`, and then lead to `SU = 0`, and we can
special case that to avoid creating a slashing event or updating any unbonding queues.

#### Exchange Rate adjustment

We have now updated the bonding, unbonding and claims amount. The net effect is lowering the
left hand side. To adapt, we will need to decrease the exchange rate slightly.

Remember: `hub_balance + bonded_tokens + unbonding_tokens - claims = lsd_tokens * exchange_rate`

`SU` decreases both `unbonding_tokens` and `claims` and thus cancels itself out. We only care about `SB`.
This works out to:

```
LSD * EXCH = X
LSD * EXCH' = X - SB

(EXCH' / EXCH) = (X - SB) / X

EXCH' =  EXCH * (1 - SB/X)
```

Make sure to save the updated exchange rate.

### Responding to Tombstoning

TL;DR: If this is a double sign slash, we remove from the active set, split its percentages
among the remainder, and ensure it is blacklisted and ignored in all future gauge updates
(splitting its  shares among the rest). We also want to trigger an automatic unbonding of
all tokens on that validator. (Investigate rebond vs unbond)

We assume that we have detected a double-slash tombstoning in before. In addition to the adjustments
made to the token holder balances, we want to ensure we remove all delegations to this validator
and never delegate to it in the future. 

To undelegate, simply send a message to undelegate all tokens on this validator, and update the
global unbonding count. We don't bond immediately, but this will avoid future unbonds for some time.
(NB: in the future look if we can safely rebond those, but let's be safer here rather than saving
a few cents). Note the Reinvest logic should be updated to handle the case when `unbonding > claims`.
This currently fails, as [`surplus` to delegate](https://github.com/cosmorama/wynd-lsd/blob/ff28e750bdcf89841dd490a376011bed84070fa5/contracts/lsd-hub/src/contract.rs#L275)
may be greater than the balance.

For splitting the tokens, we track all tombstoned validators for the future. We then apply
`remove_tombstoned()` to the current validator set, as well as any future validators assigned
by the gauge. `remove_tombstoned()` may detect that `Vx` was tombstoned, but assigned a weight `Wx`.

```
M = 1 / (1 - Wx)
Wi' = Wi * M
Wx' = 0
```

The above process may be repeated multiple times if there were multiple tombstoned validators set.