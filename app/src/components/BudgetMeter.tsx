import { formatAmount } from "../lib/format";

interface Props {
  label: string;
  spent: bigint;
  limit: bigint;
  decimals: number;
}

/** Spent against limit. Reads straight from the delegation account. */
export function BudgetMeter({ label, spent, limit, decimals }: Props) {
  const pct =
    limit === 0n ? 0 : Math.min(100, Number((spent * 100n) / limit));
  const state = pct >= 100 ? "full" : pct >= 75 ? "high" : "ok";

  return (
    <div className="meter">
      <div className="meter-head">
        <span>{label}</span>
        <span className="mono">
          {formatAmount(spent, decimals)} / {formatAmount(limit, decimals)}
        </span>
      </div>
      <div className="meter-track">
        <div className={`meter-fill ${state}`} style={{ width: `${pct}%` }} />
      </div>
    </div>
  );
}
