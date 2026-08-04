// Shapes of the Jupiter responses we consume. Keyless, live endpoints.

export type QuoteResponse = {
  inputMint: string;
  inAmount: string;
  outputMint: string;
  outAmount: string;
  otherAmountThreshold: string;
  swapMode: string;
  slippageBps: number;
  priceImpactPct: string;
  routePlan: {
    swapInfo: {
      ammKey: string;
      label: string;
      inputMint: string;
      outputMint: string;
      inAmount: string;
      outAmount: string;
    };
    percent: number;
  }[];
  swapUsdValue?: string;
  contextSlot?: number;
  timeTaken?: number;
};

export type TokenInfo = {
  id: string;
  name: string;
  symbol: string;
  icon?: string;
  decimals: number;
  usdPrice?: number;
  holderCount?: number;
  isVerified?: boolean;
  organicScore?: number;
  liquidity?: number;
};

export type QuoteResult =
  | { ok: true; quote: QuoteResponse }
  | { ok: false; error: string };

export async function fetchQuote(params: {
  inputMint: string;
  outputMint: string;
  amount: string;
  slippageBps: number;
}): Promise<QuoteResult> {
  const q = new URLSearchParams({
    inputMint: params.inputMint,
    outputMint: params.outputMint,
    amount: params.amount,
    slippageBps: String(params.slippageBps),
  });
  const res = await fetch(`/api/quote?${q.toString()}`, { cache: "no-store" });
  const data = await res.json();
  if (!res.ok || data?.error) {
    return { ok: false, error: data?.error ?? `Quote failed (${res.status})` };
  }
  return { ok: true, quote: data as QuoteResponse };
}

export async function fetchTokenInfo(query: string): Promise<TokenInfo | null> {
  const res = await fetch(`/api/token?query=${encodeURIComponent(query)}`, {
    cache: "no-store",
  });
  if (!res.ok) return null;
  const data = await res.json();
  if (Array.isArray(data) && data.length > 0) return data[0] as TokenInfo;
  return null;
}

export function routeLabels(quote: QuoteResponse): string {
  const labels = quote.routePlan?.map((r) => r.swapInfo.label) ?? [];
  const unique = Array.from(new Set(labels));
  return unique.length ? unique.join(" · ") : "direct";
}
