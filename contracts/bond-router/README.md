# Bond Router

This is an intelligent router for [lsd-hub](../lsd-hub) bonding actions.
It exposes to the user the same `Bond{}` execution action as the lsd-hub itself.
However, behind the scenes, it will query a staking swap pool for that asset, and
determine the best price for aquiring the lsd asset - buying on the market, bonding directly,
or a combination of both.

It will attempt to swap until the spot price is the same as the exchange rate we get from bonding.
Any remaining tokens will be bonded directly.  When instantiating, we must set the address of the
`lsd-hub` as well as the address of the staking swap pool to use. 
