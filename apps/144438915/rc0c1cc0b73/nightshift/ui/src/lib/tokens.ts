export type KnownToken = {
  mint: string;
  symbol: string;
  name: string;
  decimals: number;
};

// Canonical mints. Verified against Jupiter's token search on build.
export const USDC: KnownToken = {
  mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
  symbol: "USDC",
  name: "USD Coin",
  decimals: 6,
};

export const SOL: KnownToken = {
  mint: "So11111111111111111111111111111111111111112",
  symbol: "SOL",
  name: "Wrapped SOL",
  decimals: 9,
};

// A deliberately unverified lookalike mint. Shares the "SOL" ticker but is not
// the canonical wrapped-SOL mint. Used to seed the blocked-action demo: the
// envelope allowlists by MINT, not by symbol, so a ticker collision is refused.
export const LOOKALIKE_SOL: KnownToken = {
  mint: "5oLx7uPKv9C4pTaoT9dJ8Kf9V5oLLookAliKemintXXXX",
  symbol: "SOL",
  name: "Unverified lookalike (SOL)",
  decimals: 9,
};

export const KNOWN_BY_MINT: Record<string, KnownToken> = {
  [USDC.mint]: USDC,
  [SOL.mint]: SOL,
  [LOOKALIKE_SOL.mint]: LOOKALIKE_SOL,
};

export function toBaseUnits(uiAmount: number, decimals: number): string {
  // Avoid float drift by working in integer string space.
  const [whole, frac = ""] = uiAmount.toString().split(".");
  const fracPadded = (frac + "0".repeat(decimals)).slice(0, decimals);
  const combined = `${whole}${fracPadded}`.replace(/^0+(?=\d)/, "");
  return combined === "" ? "0" : combined;
}

export function fromBaseUnits(base: string, decimals: number): number {
  if (!base) return 0;
  const neg = base.startsWith("-");
  const digits = (neg ? base.slice(1) : base).padStart(decimals + 1, "0");
  const whole = digits.slice(0, digits.length - decimals);
  const frac = digits.slice(digits.length - decimals);
  return Number(`${neg ? "-" : ""}${whole}.${frac}`);
}
