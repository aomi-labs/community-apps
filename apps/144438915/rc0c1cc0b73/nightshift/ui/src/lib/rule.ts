import { KnownToken, USDC, SOL } from "./tokens";

export type Trigger =
  | { kind: "schedule"; cadence: "daily" | "hourly" | "weekly"; label: string }
  | { kind: "price"; token: KnownToken; direction: "below" | "above"; usd: number; label: string };

export type ParsedRule = {
  raw: string;
  ok: boolean;
  reason?: string;
  amountUsd: number;
  inputToken: KnownToken;
  outputToken: KnownToken;
  trigger: Trigger;
  summary: string;
};

/**
 * A small, deterministic parser for the demo. It reads a handful of plain
 * English shapes into a structured action. It is NOT an LLM — it never guesses
 * intent it cannot see, and it rejects rules it does not understand rather than
 * inventing one. Everything downstream (quote, envelope check) runs on the
 * structured output, not the prose.
 */
export function parseRule(input: string): ParsedRule {
  const raw = input.trim();
  const text = raw.toLowerCase();

  const fail = (reason: string): ParsedRule => ({
    raw,
    ok: false,
    reason,
    amountUsd: 0,
    inputToken: USDC,
    outputToken: SOL,
    trigger: { kind: "schedule", cadence: "daily", label: "—" },
    summary: "",
  });

  // For the demo we buy SOL with USDC. The output asset is SOL when named.
  const buysSol = /\bsol\b/.test(text);
  if (!buysSol) return fail("This demo trades SOL only. Name SOL as the asset.");

  const inputToken = USDC;
  const outputToken = SOL;

  // Price trigger, e.g. "if SOL drops under $120", "if SOL falls below 100".
  // Captured first so its number is not mistaken for the buy amount.
  const priceTrig = text.match(
    /(?:under|below|drops?\s+(?:under|below|to)?|falls?\s+(?:under|below|to)?|<)\s*\$?\s?(\d+(?:\.\d+)?)/,
  );
  const aboveTrig = text.match(
    /(?:over|above|rises?\s+(?:over|above|to)?|>)\s*\$?\s?(\d+(?:\.\d+)?)/,
  );
  const triggerUsd = priceTrig
    ? Number(priceTrig[1])
    : aboveTrig
      ? Number(aboveTrig[1])
      : null;

  // Buy amount: prefer the number attached to a spend verb, so the trigger
  // price is never read as the amount.
  const buyMatch = text.match(
    /(?:buy|put|invest|add|stack|dca|spend)\s+(?:in\s+)?\$?\s?(\d+(?:\.\d+)?)/,
  );
  const allDollars = Array.from(
    text.matchAll(/\$\s?(\d+(?:\.\d+)?)|(\d+(?:\.\d+)?)\s*(?:dollars|usd|usdc|bucks)/g),
  ).map((m) => Number(m[1] ?? m[2]));

  let amountUsd: number | null = null;
  if (buyMatch) amountUsd = Number(buyMatch[1]);
  else if (triggerUsd != null)
    amountUsd = allDollars.find((d) => d !== triggerUsd) ?? null;
  else amountUsd = allDollars[0] ?? null;

  if (amountUsd == null) return fail("No dollar amount found. Try “put $20 into SOL”.");
  if (!(amountUsd > 0)) return fail("Amount must be greater than zero.");

  if (priceTrig && /\bif\b/.test(text)) {
    const usd = Number(priceTrig[1]);
    return {
      raw,
      ok: true,
      amountUsd,
      inputToken,
      outputToken,
      trigger: {
        kind: "price",
        token: SOL,
        direction: "below",
        usd,
        label: `when SOL is below $${usd}`,
      },
      summary: `Buy $${amountUsd} of SOL with USDC when SOL drops under $${usd}`,
    };
  }
  if (aboveTrig && /\bif\b/.test(text)) {
    const usd = Number(aboveTrig[1]);
    return {
      raw,
      ok: true,
      amountUsd,
      inputToken,
      outputToken,
      trigger: {
        kind: "price",
        token: SOL,
        direction: "above",
        usd,
        label: `when SOL is above $${usd}`,
      },
      summary: `Buy $${amountUsd} of SOL with USDC when SOL rises over $${usd}`,
    };
  }

  // Cadence.
  let cadence: "daily" | "hourly" | "weekly" = "daily";
  if (/\bhour|hourly\b/.test(text)) cadence = "hourly";
  else if (/\bweek|weekly\b/.test(text)) cadence = "weekly";

  const cadenceLabel =
    cadence === "hourly" ? "every hour" : cadence === "weekly" ? "every week" : "every day";

  return {
    raw,
    ok: true,
    amountUsd,
    inputToken,
    outputToken,
    trigger: { kind: "schedule", cadence, label: cadenceLabel },
    summary: `Buy $${amountUsd} of SOL with USDC ${cadenceLabel}`,
  };
}

export const EXAMPLE_RULES = [
  "Every day, put $20 into SOL",
  "If SOL drops under $60, buy $15 of SOL",
  "Every hour, put $5 into SOL",
];
