import { NextRequest, NextResponse } from "next/server";

const JUP = "https://lite-api.jup.ag/swap/v1/swap";

export const dynamic = "force-dynamic";

// Builds a real, unsigned Jupiter swap transaction for a connected wallet.
// The panel hands this to Phantom for signing. Broadcast is a separate,
// explicit user action — this route only stages the transaction.
export async function POST(req: NextRequest) {
  let body: unknown;
  try {
    body = await req.json();
  } catch {
    return NextResponse.json({ error: "Invalid JSON body" }, { status: 400 });
  }

  const { quoteResponse, userPublicKey } = (body ?? {}) as {
    quoteResponse?: unknown;
    userPublicKey?: string;
  };

  if (!quoteResponse || !userPublicKey) {
    return NextResponse.json(
      { error: "quoteResponse and userPublicKey are required" },
      { status: 400 },
    );
  }

  try {
    const res = await fetch(JUP, {
      method: "POST",
      headers: { "content-type": "application/json", accept: "application/json" },
      cache: "no-store",
      body: JSON.stringify({
        quoteResponse,
        userPublicKey,
        wrapAndUnwrapSol: true,
        dynamicComputeUnitLimit: true,
      }),
    });
    const data = await res.json();
    if (!res.ok) {
      return NextResponse.json(
        { error: data?.error ?? `Jupiter swap build failed (${res.status})` },
        { status: res.status },
      );
    }
    return NextResponse.json(data);
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Swap build failed" },
      { status: 502 },
    );
  }
}
