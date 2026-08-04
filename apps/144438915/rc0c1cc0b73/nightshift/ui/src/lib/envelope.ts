import { KnownToken, USDC, SOL } from "./tokens";

/**
 * The permission envelope. In production these bounds are fixed at build time
 * by an Aomi app's namespace permissions — the agent cannot request powers it
 * was not compiled with. Here we model the same shape and enforce it in code.
 *
 * Namespace permissions this app was built with:
 *   svm-reads      — read balances, prices, token metadata
 *   svm-tx-quote   — request routes/quotes from a venue
 *   svm-tx-stage   — build an unsigned transaction (simulate-before-sign)
 *   (svm-tx-broadcast is NOT granted to the model; see `signing`)
 */
export type Namespace =
  | "svm-reads"
  | "svm-tx-quote"
  | "svm-tx-stage"
  | "svm-tx-broadcast";

export type Envelope = {
  namespaces: Namespace[];
  // The only tokens the agent may spend or receive. Allowlisted by MINT.
  allowedTokens: KnownToken[];
  // Hard ceiling on the value of any single action.
  maxPerActionUsd: number;
  // Ceiling on cumulative spend within the schedule window.
  maxDailyUsd: number;
  // The only venue the agent may route through.
  allowedVenues: string[];
  // Slippage the agent may accept, in basis points.
  maxSlippageBps: number;
};

// Who signs and who submits are the two decisions the agent never makes.
// This is the kernel policy, not a model choice.
export type SigningPolicy = {
  signer: "wallet" | "delegated-grant" | "denied";
  submitter: "wallet" | "venue" | "aomi";
};

export const DEFAULT_ENVELOPE: Envelope = {
  namespaces: ["svm-reads", "svm-tx-quote", "svm-tx-stage"],
  allowedTokens: [USDC, SOL],
  maxPerActionUsd: 50,
  maxDailyUsd: 150,
  allowedVenues: ["Jupiter aggregator"],
  maxSlippageBps: 50,
};

export type ProposedAction = {
  inputToken: KnownToken;
  outputToken: KnownToken;
  amountUsd: number;
  venue: string;
  slippageBps: number;
};

export type Violation = {
  rule: string;
  detail: string;
};

export type Verdict = {
  allowed: boolean;
  violations: Violation[];
  // The clauses that were checked and passed — shown so the box is legible.
  passed: string[];
};

/**
 * The boundary. Deterministic, not a model call. An action is inside the
 * envelope only if every clause holds. This is the enforcement the panel shows.
 */
export function evaluate(action: ProposedAction, env: Envelope): Verdict {
  const violations: Violation[] = [];
  const passed: string[] = [];

  const inAllowed = env.allowedTokens.some(
    (t) => t.mint === action.inputToken.mint,
  );
  const outAllowed = env.allowedTokens.some(
    (t) => t.mint === action.outputToken.mint,
  );

  if (inAllowed && outAllowed) {
    passed.push("Both mints are on the allowlist");
  }
  if (!inAllowed) {
    violations.push({
      rule: "Token allowlist",
      detail: `Input mint ${short(action.inputToken.mint)} (${action.inputToken.symbol}) is not on the allowlist`,
    });
  }
  if (!outAllowed) {
    violations.push({
      rule: "Token allowlist",
      detail: `Output mint ${short(action.outputToken.mint)} (${action.outputToken.symbol}) is not on the allowlist — ticker match is not identity`,
    });
  }

  if (action.amountUsd <= env.maxPerActionUsd) {
    passed.push(`Size $${action.amountUsd} is within the $${env.maxPerActionUsd} per-action ceiling`);
  } else {
    violations.push({
      rule: "Per-action ceiling",
      detail: `$${action.amountUsd} exceeds the $${env.maxPerActionUsd} max per action`,
    });
  }

  if (env.allowedVenues.includes(action.venue)) {
    passed.push(`Venue "${action.venue}" is allowed`);
  } else {
    violations.push({
      rule: "Venue allowlist",
      detail: `"${action.venue}" is not an allowed venue`,
    });
  }

  if (action.slippageBps <= env.maxSlippageBps) {
    passed.push(`Slippage ${action.slippageBps}bps is within the ${env.maxSlippageBps}bps ceiling`);
  } else {
    violations.push({
      rule: "Slippage ceiling",
      detail: `${action.slippageBps}bps exceeds the ${env.maxSlippageBps}bps ceiling`,
    });
  }

  return { allowed: violations.length === 0, violations, passed };
}

function short(mint: string): string {
  return `${mint.slice(0, 4)}…${mint.slice(-4)}`;
}
