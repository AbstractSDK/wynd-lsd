import {
  ExecuteResult,
  SigningCosmWasmClient,
} from '@cosmjs/cosmwasm-stargate/build/signingcosmwasmclient';

import { DirectSecp256k1HdWallet } from '@cosmjs/proto-signing';
import { GasPrice, calculateFee } from '@cosmjs/stargate';
import { readFileSync } from 'fs';

import { alice, ENDPOINT } from './constants';


export async function getCosmWasmClient(mnemonic: string = alice.mnemonic) {
  const gasPrice = GasPrice.fromString("0.025ucosm");
  const wallet = await DirectSecp256k1HdWallet.fromMnemonic(mnemonic, { prefix: "wasm" });
  const client = await SigningCosmWasmClient.connectWithSigner(ENDPOINT, wallet);
  return { gasPrice, client };
}

/**
 * Encode a JSON object to base64 string
 */
export function encodeBase64(obj: object | string | number) {
  return btoa(JSON.stringify(obj));
}

/**
* Encode a string to UTF8 array
*/
export function encodeUtf8(str: string) {
  const encoder = new TextEncoder();
  return Array.from(encoder.encode(str));
}