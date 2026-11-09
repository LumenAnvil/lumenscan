import { useRef, useState } from "react";

import { InterfaceView } from "./InterfaceView";
import {
  fetchContractSpec,
  isValidContractId,
  NETWORKS,
  specFromWasm,
} from "./inspect";
import { buildInterface, type Interface } from "./model";

interface Result {
  source: string;
  iface: Interface;
}

type Status =
  | { kind: "idle" }
  | { kind: "loading" }
  | { kind: "error"; message: string }
  | { kind: "done"; result: Result };

export function App() {
  const [contractId, setContractId] = useState("");
  const [networkId, setNetworkId] = useState(NETWORKS[0].id);
  const [rpcUrl, setRpcUrl] = useState(NETWORKS[0].rpcUrl);
  const [status, setStatus] = useState<Status>({ kind: "idle" });
  const fileInput = useRef<HTMLInputElement>(null);

  const network = NETWORKS.find((n) => n.id === networkId) ?? NETWORKS[0];

  function selectNetwork(id: string) {
    setNetworkId(id);
    const n = NETWORKS.find((x) => x.id === id);
    if (n) setRpcUrl(n.rpcUrl);
  }

  async function inspectNetwork(event: React.FormEvent) {
    event.preventDefault();
    const id = contractId.trim();
    if (!isValidContractId(id)) {
      setStatus({
        kind: "error",
        message:
          "That is not a valid contract ID. It should start with C and be 56 characters long.",
      });
      return;
    }

    setStatus({ kind: "loading" });
    try {
      const entries = await fetchContractSpec(id, {
        rpcUrl,
        passphrase: network.passphrase,
      });
      setStatus({
        kind: "done",
        result: { source: `contract ${id}`, iface: buildInterface(entries) },
      });
    } catch (err) {
      setStatus({ kind: "error", message: describe(err) });
    }
  }

  async function inspectFile(file: File) {
    setStatus({ kind: "loading" });
    try {
      const bytes = new Uint8Array(await file.arrayBuffer());
      const entries = specFromWasm(bytes);
      setStatus({
        kind: "done",
        result: { source: file.name, iface: buildInterface(entries) },
      });
    } catch (err) {
      setStatus({ kind: "error", message: describe(err) });
    }
  }

  return (
    <div className="app">
      <header>
        <h1>Lumenscan</h1>
        <p className="tagline">
          Inspect a deployed Soroban contract&rsquo;s interface — decoded from
          the contract&rsquo;s own WASM, right in your browser.
        </p>
      </header>

      <form className="controls" onSubmit={inspectNetwork}>
        <div className="row">
          <input
            type="text"
            className="contract-id"
            placeholder="Contract ID (C…)"
            value={contractId}
            onChange={(e) => setContractId(e.target.value)}
            autoCapitalize="characters"
            spellCheck={false}
            aria-label="Contract ID"
          />
          <button type="submit" disabled={status.kind === "loading"}>
            {status.kind === "loading" ? "Inspecting…" : "Inspect"}
          </button>
        </div>

        <div className="row options">
          <label>
            Network
            <select
              value={networkId}
              onChange={(e) => selectNetwork(e.target.value)}
            >
              {NETWORKS.map((n) => (
                <option key={n.id} value={n.id}>
                  {n.label}
                </option>
              ))}
            </select>
          </label>
          <label className="rpc">
            RPC URL
            <input
              type="text"
              value={rpcUrl}
              onChange={(e) => setRpcUrl(e.target.value)}
              spellCheck={false}
            />
          </label>
        </div>

        <div className="row file-row">
          <span className="or">or</span>
          <button
            type="button"
            className="secondary"
            onClick={() => fileInput.current?.click()}
          >
            Inspect a local .wasm file
          </button>
          <input
            ref={fileInput}
            type="file"
            accept=".wasm,application/wasm"
            hidden
            onChange={(e) => {
              const file = e.target.files?.[0];
              if (file) void inspectFile(file);
              e.target.value = "";
            }}
          />
        </div>
      </form>

      <main>
        {status.kind === "idle" && (
          <p className="hint">
            Paste a contract ID, or load a <code>.wasm</code> build, to see its
            functions, types and events.
          </p>
        )}
        {status.kind === "loading" && <p className="hint">Fetching…</p>}
        {status.kind === "error" && (
          <div className="error" role="alert">
            {status.message}
          </div>
        )}
        {status.kind === "done" && (
          <>
            <p className="source">
              <code>{status.result.source}</code>
            </p>
            <InterfaceView iface={status.result.iface} />
          </>
        )}
      </main>

      <footer>
        <p>
          A companion to the{" "}
          <a href="https://github.com/LumenAnvil/lumenscan">lumenscan</a> CLI.
          Read-only: it never deploys, invokes, or changes anything.
        </p>
      </footer>
    </div>
  );
}

function describe(err: unknown): string {
  if (err instanceof Error) return err.message;
  return String(err);
}
