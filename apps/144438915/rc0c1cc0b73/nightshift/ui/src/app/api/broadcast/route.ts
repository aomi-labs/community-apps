import { NextRequest, NextResponse } from "next/server";

// Public mainnet RPC. Rate-limited, keyless — fine for a single demo send.
const RPC = "https://api.mainnet-beta.solana.com";

export const dynamic = "force-dynamic";

// Broadcasts a signed transaction. This is the ONLY place funds move, and it
// only runs when the user explicitly clicks Broadcast after Phantom has signed.
export async function POST(req: NextRequest) {
  let body: unknown;
  try {
    body = await req.json();
  } catch {
    return NextResponse.json({ error: "Invalid JSON body" }, { status: 400 });
  }
  const { signedTransaction } = (body ?? {}) as { signedTransaction?: string };
  if (!signedTransaction) {
    return NextResponse.json(
      { error: "signedTransaction (base64) is required" },
      { status: 400 },
    );
  }
  try {
    const res = await fetch(RPC, {
      method: "POST",
      headers: { "content-type": "application/json" },
      cache: "no-store",
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: 1,
        method: "sendTransaction",
        params: [signedTransaction, { encoding: "base64", skipPreflight: false }],
      }),
    });
    const data = await res.json();
    if (data?.error) {
      return NextResponse.json(
        { error: data.error?.message ?? "RPC rejected the transaction" },
        { status: 400 },
      );
    }
    return NextResponse.json({ signature: data.result });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Broadcast failed" },
      { status: 502 },
    );
  }
}
