import { NextRequest, NextResponse } from "next/server";

const JUP = "https://lite-api.jup.ag/swap/v1/quote";

export const dynamic = "force-dynamic";

export async function GET(req: NextRequest) {
  const { searchParams } = new URL(req.url);
  const inputMint = searchParams.get("inputMint");
  const outputMint = searchParams.get("outputMint");
  const amount = searchParams.get("amount");
  const slippageBps = searchParams.get("slippageBps") ?? "50";

  if (!inputMint || !outputMint || !amount) {
    return NextResponse.json(
      { error: "inputMint, outputMint and amount are required" },
      { status: 400 },
    );
  }

  const q = new URLSearchParams({
    inputMint,
    outputMint,
    amount,
    slippageBps,
    restrictIntermediateTokens: "true",
  });

  try {
    const res = await fetch(`${JUP}?${q.toString()}`, {
      headers: { accept: "application/json" },
      cache: "no-store",
    });
    const data = await res.json();
    if (!res.ok) {
      return NextResponse.json(
        { error: data?.error ?? `Jupiter quote failed (${res.status})` },
        { status: res.status },
      );
    }
    return NextResponse.json(data);
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Quote request failed" },
      { status: 502 },
    );
  }
}
