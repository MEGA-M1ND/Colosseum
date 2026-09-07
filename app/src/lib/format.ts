export function shortKey(key: { toBase58(): string } | string): string {
  const s = typeof key === "string" ? key : key.toBase58();
  return `${s.slice(0, 4)}…${s.slice(-4)}`;
}

/** Token amounts are stored as integers; render them with the mint's decimals. */
export function formatAmount(raw: bigint, decimals: number): string {
  const negative = raw < 0n;
  const abs = negative ? -raw : raw;
  const base = 10n ** BigInt(decimals);
  const whole = abs / base;
  const frac = abs % base;
  const fracStr = frac
    .toString()
    .padStart(decimals, "0")
    .replace(/0+$/, "");
  return `${negative ? "-" : ""}${whole}${fracStr ? `.${fracStr}` : ""}`;
}

export function toRaw(input: string, decimals: number): bigint {
  const [whole, frac = ""] = input.trim().split(".");
  const padded = (frac + "0".repeat(decimals)).slice(0, decimals);
  return BigInt(whole || "0") * 10n ** BigInt(decimals) + BigInt(padded || "0");
}

export function relativeTime(unixSeconds: bigint): string {
  const delta = Number(unixSeconds) - Math.floor(Date.now() / 1000);
  if (delta <= 0) return "expired";
  const units: [number, string][] = [
    [86400, "day"],
    [3600, "hour"],
    [60, "minute"],
  ];
  for (const [secs, label] of units) {
    if (delta >= secs) {
      const n = Math.floor(delta / secs);
      return `${n} ${label}${n === 1 ? "" : "s"}`;
    }
  }
  return `${delta}s`;
}
