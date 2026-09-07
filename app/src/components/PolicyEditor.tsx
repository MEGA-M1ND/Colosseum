import { useState } from "react";
import { PublicKey } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import type { Policy } from "../lib/program";
import type { DelegationAccount } from "../lib/accounts";
import { relativeTime, toRaw } from "../lib/format";
import { BudgetMeter } from "./BudgetMeter";

interface Props {
  decimals: number;
  delegation: DelegationAccount | null;
  disabled: boolean;
  onGrant: (p: Policy) => void;
  onUpdate: (p: Policy) => void;
  onRevoke: () => void;
}

export function PolicyEditor({
  decimals,
  delegation,
  disabled,
  onGrant,
  onUpdate,
  onRevoke,
}: Props) {
  const [perTx, setPerTx] = useState("10");
  const [total, setTotal] = useState("100");
  const [windowMinutes, setWindowMinutes] = useState("60");
  const [windowLimit, setWindowLimit] = useState("50");
  const [expiryHours, setExpiryHours] = useState("24");
  const [allowInvoke, setAllowInvoke] = useState(false);

  const build = (): Policy => ({
    perTxLimit: toRaw(perTx, decimals),
    totalLimit: toRaw(total, decimals),
    // A duration with no limit would silently allow everything, so the program
    // requires both or neither.
    windowDuration: BigInt(Number(windowMinutes) * 60),
    windowLimit: toRaw(windowLimit, decimals),
    expiresAt: BigInt(
      Math.floor(Date.now() / 1000) + Number(expiryHours) * 3600,
    ),
    allowedDestinations: [],
    allowedPrograms: allowInvoke ? [new PublicKey(TOKEN_PROGRAM_ID)] : [],
  });

  return (
    <section className="card">
      <h2>Policy</h2>
      <p className="hint">
        Enforced on-chain, re-read on every spend. Revocation takes effect on the
        agent's next instruction.
      </p>

      <div className="fields">
        <label className="field">
          <span>Per transaction</span>
          <input value={perTx} onChange={(e) => setPerTx(e.target.value)} />
        </label>
        <label className="field">
          <span>Lifetime total</span>
          <input value={total} onChange={(e) => setTotal(e.target.value)} />
        </label>
        <label className="field">
          <span>Window (minutes)</span>
          <input
            value={windowMinutes}
            onChange={(e) => setWindowMinutes(e.target.value)}
          />
        </label>
        <label className="field">
          <span>Per window</span>
          <input
            value={windowLimit}
            onChange={(e) => setWindowLimit(e.target.value)}
          />
        </label>
        <label className="field">
          <span>Expires in (hours)</span>
          <input
            value={expiryHours}
            onChange={(e) => setExpiryHours(e.target.value)}
          />
        </label>
      </div>

      <label className="check">
        <input
          type="checkbox"
          checked={allowInvoke}
          onChange={(e) => setAllowInvoke(e.target.checked)}
        />
        <span>
          Allow arbitrary calls to the token program
          <em>
            Off by default. Even when on, spending is still bounded by the caps
            above — the program measures what left the vault.
          </em>
        </span>
      </label>

      <div className="row">
        <button onClick={() => onGrant(build())} disabled={disabled}>
          Grant budget
        </button>
        <button
          className="secondary"
          onClick={() => onUpdate(build())}
          disabled={disabled || !delegation}
        >
          Update
        </button>
        <button
          className="danger"
          onClick={onRevoke}
          disabled={disabled || !delegation || delegation.revoked}
        >
          Revoke
        </button>
      </div>

      {delegation && (
        <div className="live">
          <div className="live-head">
            <span>Live delegation</span>
            {delegation.revoked ? (
              <span className="tag revoked">revoked</span>
            ) : (
              <span className="tag active">
                expires in {relativeTime(delegation.expiresAt)}
              </span>
            )}
          </div>
          <BudgetMeter
            label="Lifetime"
            spent={delegation.totalSpent}
            limit={delegation.totalLimit}
            decimals={decimals}
          />
          {delegation.windowDuration > 0n && (
            <BudgetMeter
              label="This window"
              spent={delegation.windowSpent}
              limit={delegation.windowLimit}
              decimals={decimals}
            />
          )}
        </div>
      )}
    </section>
  );
}
