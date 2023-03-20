import "jasmine";

import { encodeBase64, getCosmWasmClient } from "../src/index";
import { coin } from '@cosmjs/proto-signing';
import { calculateFee } from '@cosmjs/stargate';
import { CONFIG_QUERY, BOND_EXECUTE_MSG, CLAIM_EXECUTE_MSG, alice, bob, DEFAULT_COMISSION, DEFAULT_UNBOND_PEROID, DEFAULT_EPOCH_PEROID, DELAY_EPOCH, GAS_INSTANTIATE_COST, GAS_EXECUTE_COST } from "../src/constants";
import { SpecReporter, StacktraceOption } from 'jasmine-spec-reporter';

// Make Jasmine output prettier and more verbose
jasmine.getEnv().addReporter(new SpecReporter({ spec: { displayStacktrace: StacktraceOption.PRETTY } }));
const WYND_LSD_HUB = "wasm1wug8sewp6cedgkmrmvhl3lf3tulagm9hnvy8p0rppz9yjw0g4wtqhs9hr8";

// Extend the default timeout as we need to wait a number of seconds for each block, and at least one block per transaction
jasmine.DEFAULT_TIMEOUT_INTERVAL = 30000;

// Helper that avoids using setTimeout and callbacks as much 
const delay = ms => new Promise(res => setTimeout(res, ms));

describe("The LSD Hub", function () {
    it("should be deployed with the right values", async function () {
        const { client } = await getCosmWasmClient();

        const result = await client.queryContractSmart(WYND_LSD_HUB, CONFIG_QUERY);

        expect(result.owner).toBe(alice.address0);
        expect(result.commission).toBe(DEFAULT_COMISSION);
        expect(result.unbond_period).toBe(DEFAULT_UNBOND_PEROID);
        expect(result.epoch_period).toBe(DEFAULT_EPOCH_PEROID);
    });

    it("should not be able to increase the discount past 50%", async function () {
        const { gasPrice, client } = await getCosmWasmClient();
        const executeFee = calculateFee(GAS_EXECUTE_COST, gasPrice);

        // UpdateLiquidityDiscount 
        const updateLiquidityDiscountResult = await client.execute(alice.address0, WYND_LSD_HUB, {
            update_liquidity_discount: {
                new_discount: "0.51"
            }
        }, executeFee, "", []).catch((err) => {
            // Expect the error to contain The given liquidity discount was invalid
            expect(err.message).toContain("The given liquidity discount was invalid");
        });
        const newTargetValue = await client.queryContractSmart(WYND_LSD_HUB, { target_value: {} });
        // Also verify the target value is still 0.05
        expect(parseFloat(newTargetValue.target_value).toFixed(2)).toBe('0.95');
    });

    it("should be able to increase the discount under 50%", async function () {
        const { gasPrice, client } = await getCosmWasmClient();
        const executeFee = calculateFee(GAS_EXECUTE_COST, gasPrice);

        // UpdateLiquidityDiscount 
        const updateLiquidityDiscountResult = await client.execute(alice.address0, WYND_LSD_HUB, {
            update_liquidity_discount: {
                new_discount: "0.49"
            }
        }, executeFee, "", []);
        // Get liqudity discount from the contract
        const result = await client.queryContractSmart(WYND_LSD_HUB, CONFIG_QUERY);

        const newTargetValue = await client.queryContractSmart(WYND_LSD_HUB, { target_value: {} });
        expect(parseFloat(newTargetValue.target_value).toFixed(2)).toBe('0.51');

        // Update it back to normal for other tests 
        // UpdateLiquidityDiscount 
        await client.execute(alice.address0, WYND_LSD_HUB, {
            update_liquidity_discount: {
                new_discount: "0.05"
            }
        }, executeFee, "", []);

        const tvAfterSecondDiscount = await client.queryContractSmart(WYND_LSD_HUB, { target_value: {} });
        expect(parseFloat(tvAfterSecondDiscount.target_value).toFixed(2)).toBe('0.95');
    });

    it("should be able to handle a basic minting case", async function () {

        const { gasPrice, client } = await getCosmWasmClient();
        const claim_amount = 100000;

        const configQueryResult = await client.queryContractSmart(WYND_LSD_HUB, CONFIG_QUERY);
        const WYND_LSD_TOKEN = configQueryResult.token_contract;

        const executeFee = calculateFee(GAS_EXECUTE_COST, gasPrice);

        // Take a previous balance for Alice and  Bob just incase he already has deposits 
        const bobsPreviousBalance = await client.queryContractSmart(WYND_LSD_TOKEN, { balance: { address: bob.address0 } });
        const alicePreviousBalance = await client.queryContractSmart(WYND_LSD_TOKEN, { balance: { address: alice.address0 } });

        const executeResult = await client.execute(alice.address0, WYND_LSD_HUB, BOND_EXECUTE_MSG, executeFee, "", [coin(claim_amount, "ustake")]);
        // Verify the token contract is the same one given to the user in the result 
        // Once the message structure is finalized this events position will never change but may be better to remove this check
        expect(configQueryResult.token_contract).toBe(executeResult.events[13].attributes[0].value);

        // Note we need a second client setup with a second mnemonic to be able to claim as a second user 
        const { client: bobsClient } = await getCosmWasmClient(bob.mnemonic);
        await bobsClient.execute(bob.address0, WYND_LSD_HUB, BOND_EXECUTE_MSG, executeFee, "", [coin(claim_amount, "ustake")]);


        // Query the WYND_LSD_HUB contract for the balance of bob.address0 and alice.address0
        const bobBalance = await bobsClient.queryContractSmart(WYND_LSD_TOKEN, { balance: { address: bob.address0 } });
        const aliceBalance = await bobsClient.queryContractSmart(WYND_LSD_TOKEN, { balance: { address: alice.address0 } });

        // Get the exchange rate, todo: requires wasm update 
        // const exchangeRate = await bobsClient.queryContractSmart(WYND_LSD_HUB, {exchange_rate: {}});
        expect(bobBalance.balance - bobsPreviousBalance.balance).toBe(claim_amount);
        expect(aliceBalance.balance - alicePreviousBalance.balance).toBe(claim_amount);
    });

    it("should be able to process a claim", async function () {

        const { gasPrice, client } = await getCosmWasmClient();
        const claim_amount = 100000;

        const configQueryResult = await client.queryContractSmart(WYND_LSD_HUB, CONFIG_QUERY);
        const executeFee = calculateFee(GAS_EXECUTE_COST, gasPrice);
        const WYND_LSD_TOKEN = configQueryResult.token_contract;

        await client.execute(alice.address0, WYND_LSD_TOKEN, {
            send: {
                contract: WYND_LSD_HUB,
                amount: `${claim_amount}`,
                msg: encodeBase64({
                    unbond: {
                    }
                })
            }
        }, executeFee, "", []);

        // Query the LSD HUb and verify there is a claim for alice.address0
        const claimQueryResult = await client.queryContractSmart(WYND_LSD_HUB, { claims: { address: alice.address0 } });

        // Query user balance of native token ustake for later 
        const nativeTokenBalanceBeforeClaim = await client.getBalance(alice.address0, "ustake");
        await client.execute(alice.address0, WYND_LSD_HUB, CLAIM_EXECUTE_MSG, executeFee, "", []);

        // Wait 5s for an epoch 
        await delay(DELAY_EPOCH);

        const newClaimsResult = await client.queryContractSmart(WYND_LSD_HUB, { claims: { address: alice.address0 } });
        const nativeTokenBalanceAfterClaim = await client.getBalance(alice.address0, "ustake");
        // Verify we have one less claim at least.
        // If running scripts over and over extra claims may be present so this prevents it all falling down in that case.
        expect(claimQueryResult.claims.length - 1).toBe(newClaimsResult.claims.length);
        // Convert nativeTokenBalanceBeforeClaim.amount to a number and nativeTokenBalanceAfterClaim to a number, expect it to be the claim_amount difference
        expect(Number(nativeTokenBalanceAfterClaim.amount) - Number(nativeTokenBalanceBeforeClaim.amount)).toBe(claim_amount);

    });

    it("should do a delegation after a reinvest", async function () {

        const { gasPrice, client } = await getCosmWasmClient();
        const claim_amount = 100000;

        const configQueryResult = await client.queryContractSmart(WYND_LSD_HUB, CONFIG_QUERY);
        const executeFee = calculateFee(GAS_EXECUTE_COST, gasPrice);
        const WYND_LSD_TOKEN = configQueryResult.token_contract;

        const executeResult = await client.execute(alice.address0, WYND_LSD_HUB, BOND_EXECUTE_MSG, executeFee, "", [coin(claim_amount, "ustake")]);

        // Note: Alice is currently 'treasury' if that changes so must this 
        const nativeTokenBalanceBeforeReinvest = await client.getBalance(alice.address0, "ustake");

        // Wait 5s for an epoch 
        await delay(DELAY_EPOCH);

        // Call reinvest
        const reInvestResult = await client.execute(alice.address0, WYND_LSD_HUB, { reinvest: {} }, executeFee, "", []);
        // Expect an amount of 200000ustake to be delegated to the only validator in the hub 
        expect(reInvestResult.events[12].attributes[1].value).toBe('200000ustake');
    });


    it("should reinvest and update exchange rate", async function () {

        const { gasPrice, client } = await getCosmWasmClient(bob.mnemonic);
        const claim_amount = 100000;

        const configQueryResult = await client.queryContractSmart(WYND_LSD_HUB, CONFIG_QUERY);
        const executeFee = calculateFee(GAS_EXECUTE_COST, gasPrice);
        const WYND_LSD_TOKEN = configQueryResult.token_contract;

        const executeResult = await client.execute(bob.address0, WYND_LSD_HUB, BOND_EXECUTE_MSG, executeFee, "", [coin(claim_amount, "ustake")]);
        await client.execute(bob.address0, WYND_LSD_HUB, BOND_EXECUTE_MSG, executeFee, "", [coin(claim_amount, "ustake")]);
        const exchangeRateBeforeReinvest = await client.queryContractSmart(WYND_LSD_HUB, { exchange_rate: {} });
        expect(exchangeRateBeforeReinvest.exchange_rate).toBe('1');

        // Wait 5s for an epoch 
        await delay(DELAY_EPOCH);
        // Call reinvest
        const reInvestResult = await client.execute(bob.address0, WYND_LSD_HUB, { reinvest: {} }, executeFee, "", []);

        const exchangeRateAfterReinvest = await client.queryContractSmart(WYND_LSD_HUB, { exchange_rate: {} });
        expect(exchangeRateAfterReinvest.exchange_rate).toBe('1.0000125');
    });

    it("should be able to update target value and target value increases as expected", async function () {

        const { gasPrice, client } = await getCosmWasmClient();
        const claim_amount = 100000;

        const configQueryResult = await client.queryContractSmart(WYND_LSD_HUB, CONFIG_QUERY);
        const executeFee = calculateFee(GAS_EXECUTE_COST, gasPrice);
        const WYND_LSD_TOKEN = configQueryResult.token_contract;
        // Get Target Value 
        const targetValue = await client.queryContractSmart(WYND_LSD_HUB, { target_value: {} });
        // Discount is 5% on deploy so 100 - 5 0.95
        expect(parseFloat(targetValue.target_value).toFixed(2)).toBe('0.95');
        await client.execute(alice.address0, WYND_LSD_HUB, BOND_EXECUTE_MSG, executeFee, "", [coin(claim_amount, "ustake")]);
        const tvAfterBond = await client.queryContractSmart(WYND_LSD_HUB, { target_value: {} });
        // Target Value should be stay the same after a deposit 
        expect(parseFloat(targetValue.target_value).toFixed(2)).toBe(parseFloat(tvAfterBond.target_value).toFixed(2));
        // Call reinvest
        const tvAfterReinvest = await client.queryContractSmart(WYND_LSD_HUB, { target_value: {} });
        // Target Value should be stay the same after a deposit 
        expect(parseFloat(targetValue.target_value).toFixed(2)).toBe(parseFloat(tvAfterReinvest.target_value).toFixed(2));
        // Update the liquidity discount to 4%
        const updateLiquidityDiscountResult = await client.execute(alice.address0, WYND_LSD_HUB, { update_liquidity_discount: { new_discount: "0.04" } }, executeFee, "", []);
        const newTargetValue = await client.queryContractSmart(WYND_LSD_HUB, { target_value: {} });
        expect(parseFloat(newTargetValue.target_value).toFixed(2)).toBe('0.96');
        // Wait 5s for an epoch 
        await delay(DELAY_EPOCH);
        const aliceNativeBalanceBefore = await client.getBalance(alice.address0, "ustake");

        // Call reinvest
        const secondReinvest = await client.execute(alice.address0, WYND_LSD_HUB, { reinvest: {} }, executeFee, "", []);
        const tvAfterSecondReinvest = await client.queryContractSmart(WYND_LSD_HUB, { target_value: {} });
        const aliceNativeBalanceAfter = await client.getBalance(alice.address0, "ustake");

        // Target value should have increased, because of deposit values and epoch times it won't be much so no toFixed here.
        expect(parseFloat(tvAfterSecondReinvest.target_value)).toBeGreaterThan(parseFloat(newTargetValue.target_value));
        // Expect Alices balance to have increased, because of the reinvest and because Alice is the treasury she received some commission
        // expect(Number(aliceNativeBalanceBefore.amount)).toBeLessThan(Number(aliceNativeBalanceAfter.amount));
    });

    // it("commission should be gathered after a reinvest", async function () {

    //     const { gasPrice, client } = await getCosmWasmClient();
    //     const claim_amount = 100000;
    //     const executeFee = calculateFee(600_000, gasPrice);


    //     await client.execute(alice.address0, WYND_LSD_HUB, BOND_EXECUTE_MSG, executeFee, "", [coin(claim_amount, "ustake")]);
    //     // Get alice native token balance 
    //     const aliceNativeBalanceBefore = await client.getBalance(alice.address0, "ustake");
    //     const tvAfterBond = await client.queryContractSmart(WYND_LSD_HUB, { target_value: {} });
    //     // Call reinvest
    //     const tvAfterReinvest = await client.queryContractSmart(WYND_LSD_HUB, { target_value: {} });
    //     await client.execute(alice.address0, WYND_LSD_HUB, {reinvest: {}}, executeFee, "", []);
    //     await delay(5000);
    //     const aliceNativeBalanceAfter = await client.getBalance(alice.address0, "ustake");
    //     expect(Number(aliceNativeBalanceBefore.amount)).toBeGreaterThan(Number(aliceNativeBalanceAfter.amount));

    // });
});
