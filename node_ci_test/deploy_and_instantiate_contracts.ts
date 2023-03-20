#!/usr/bin/env node

/* eslint-disable @typescript-eslint/naming-convention */
import {
  ExecuteResult,
  SigningCosmWasmClient,
} from '@cosmjs/cosmwasm-stargate/build/signingcosmwasmclient';

import { DirectSecp256k1HdWallet } from '@cosmjs/proto-signing';
import { GasPrice, calculateFee } from '@cosmjs/stargate';
import {readFileSync} from 'fs';
declare var process; 
declare var __dirname;
import { getCosmWasmClient} from './src/index';
import { alice, DEFAULT_COMISSION, DEFAULT_EPOCH_PEROID, DEFAULT_UNBOND_PEROID, ENDPOINT, GAS_INSTANTIATE_COST, GAS_UPLOAD_COST } from './src/constants';
async function main() {
  const { gasPrice, client } = await getCosmWasmClient();

  var wasm = readFileSync(__dirname + "/contracts/cw20_base.wasm");
  const uploadFee = calculateFee(GAS_UPLOAD_COST, gasPrice);
  const uploadReceiptToken = await client.upload(
    alice.address0,
    wasm,
    uploadFee,
    "Upload cw20_base contract",
  );
  console.info(`cw20_base upload succeeded. Receipt: ${JSON.stringify(uploadReceiptToken)}`);

  var wasm = readFileSync(__dirname + "/contracts/wynd_lsd_hub.wasm");
  const uploadReceiptLsdHub = await client.upload(
    alice.address0,
    wasm,
    uploadFee,
    "Upload lsd-hub contract",
  );
  console.info(`lsd-hub upload succeeded. Receipt: ${JSON.stringify(uploadReceiptLsdHub)}`);

  const instantiateFee = calculateFee(GAS_INSTANTIATE_COST, gasPrice);
  const label = "WYND-LSD-HUB";
  // TODO: change this to a better address?
  const admin = alice.address0;
  {
    const { contractAddress } = await client.instantiate(
      alice.address0,
      uploadReceiptLsdHub.codeId,
      {
        // TODO: change this to a better address
        treasury: admin,
        comission: DEFAULT_COMISSION,
        owner: admin,
        validators: [["wasmvaloper1tjgue6r5kqj5dets24pwaa9u7wuzucpwfsgndk", "1.0"]],
        cw20_init: {
                cw20_code_id: uploadReceiptToken.codeId,
                label: "WYND-LSD-TOKEN",
                name: "WYND-LSD",
                symbol: "wLSD",
                decimals: 6,
                initial_balances: []
        },
        epoch_period: DEFAULT_EPOCH_PEROID,
        unbond_period: DEFAULT_UNBOND_PEROID,
        max_concurrent_unbondings: 7,
        liquidity_discount: "0.05"
      },
      label,
      instantiateFee,
      {
        memo: `Create an WYND-LSD-HUB instance.`,
        admin: admin,
      },
    );
    console.info(`Wynd lsd hub contract instantiated at ${contractAddress}`);
  }
}

main().then(
  () => {
    console.info("All done, let the coins flow.");
    process.exit(0);
  },
  (error) => {
    console.error(error);
    process.exit(1);
  },
);


