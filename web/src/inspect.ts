// Getting a contract's spec entries — from the network, or from a local .wasm.
//
// Both paths lean on the SDK: `contract.Client.from` fetches a deployed
// contract and decodes its spec; `contract.Spec.fromWasm` reads the
// `contractspecv0` custom section out of raw wasm bytes. Everything downstream
// (see model.ts) works on the resulting `xdr.ScSpecEntry[]`, so the two sources
// are interchangeable.

import { contract, StrKey, xdr } from "@stellar/stellar-sdk";

export interface Network {
  id: string;
  label: string;
  rpcUrl: string;
  passphrase: string;
}

// Passphrases are the canonical Stellar network identifiers; the default RPC
// URLs are the public endpoints. Both can be overridden in the UI.
export const NETWORKS: Network[] = [
  {
    id: "testnet",
    label: "Testnet",
    rpcUrl: "https://soroban-testnet.stellar.org",
    passphrase: "Test SDF Network ; September 2015",
  },
  {
    id: "mainnet",
    label: "Mainnet",
    rpcUrl: "https://mainnet.sorobanrpc.com",
    passphrase: "Public Global Stellar Network ; September 2015",
  },
];

export function isValidContractId(id: string): boolean {
  return StrKey.isValidContract(id.trim());
}

/** Fetch and decode a deployed contract's spec entries over RPC. */
export async function fetchContractSpec(
  contractId: string,
  network: { rpcUrl: string; passphrase: string },
): Promise<xdr.ScSpecEntry[]> {
  const id = contractId.trim();
  if (!isValidContractId(id)) {
    throw new Error(
      "That is not a valid contract ID. It should start with C and be 56 characters long.",
    );
  }

  const client = await contract.Client.from({
    contractId: id,
    rpcUrl: network.rpcUrl,
    networkPassphrase: network.passphrase,
    allowHttp: network.rpcUrl.startsWith("http://"),
  });

  return client.spec.entries;
}

/** Decode spec entries from the bytes of a local .wasm build. */
export function specFromWasm(bytes: Uint8Array): xdr.ScSpecEntry[] {
  const spec = contract.Spec.fromWasm(bytes);
  return spec.entries;
}
