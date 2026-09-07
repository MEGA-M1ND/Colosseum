import { useState } from "react";
import type { PublicKey } from "@solana/web3.js";
import { formatAmount, shortKey, toRaw } from "../lib/format";

interface Props {
  vault: PublicKey | null;
  mintInput: string;
  onMintChange: (v: string) => void;
  mintValid: boolean;
  vaultAta: PublicKey | null;
  vaultBalance: bigint | null;
  decimals: number;
  busy: boolean;
  onCreateVault: () => void;
  onDeposit: (amount: bigint) => void;
}

export function VaultSetup({
  vault,
  mintInput,
  onMintChange,
  mintValid,
  vaultAta,
  vaultBalance,
  decimals,
  busy,
  onCreateVault,
  onDeposit,
}: Props) {
  const [amount, setAmount] = useState("100");

  return (
    <section className="card">
      <h2>Vault</h2>
      <p className="hint">
        Funds live in a PDA the program controls. Your wallet stays the only key
        that can change the policy or withdraw.
      </p>

      <label className="field">
        <span>Token mint</span>
        <input
          value={mintInput}
          onChange={(e) => onMintChange(e.target.value)}
          placeholder="Mint address"
          spellCheck={false}
        />
        {mintInput && !mintValid && (
          <em className="warn">Not a valid address</em>
        )}
      </label>

      <dl className="kv">
        <div>
          <dt>Vault PDA</dt>
          <dd>{vault ? shortKey(vault) : "—"}</dd>
        </div>
        <div>
          <dt>Vault token account</dt>
          <dd>{vaultAta ? shortKey(vaultAta) : "—"}</dd>
        </div>
        <div>
          <dt>Balance</dt>
          <dd className="strong">
            {vaultBalance === null
              ? "—"
              : formatAmount(vaultBalance, decimals)}
          </dd>
        </div>
      </dl>

      <div className="row">
        <button onClick={onCreateVault} disabled={busy}>
          Create vault
        </button>
      </div>

      <div className="row">
        <input
          className="amount"
          value={amount}
          onChange={(e) => setAmount(e.target.value)}
          inputMode="decimal"
        />
        <button
          onClick={() => onDeposit(toRaw(amount, decimals))}
          disabled={busy || !mintValid}
        >
          Deposit
        </button>
      </div>
    </section>
  );
}
