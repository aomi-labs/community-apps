import { NextRequest, NextResponse } from "next/server";

const JUP = "https://lite-api.jup.ag/price/v3";

export const dynamic = "force-dynamic";

export async function GET(req: NextRequest) {
  const { searchParams } = new URL(req.url);
  const ids = searchParams.get("ids");
  if (!ids) {
    return NextResponse.json({ error: "ids is required" }, { status: 400 });
  }
  try {
    const res = await fetch(`${JUP}?ids=${encodeURIComponent(ids)}`, {
      headers: { accept: "application/json" },
      cache: "no-store",
    });
    const data = await res.json();
    if (!res.ok) {
      return NextResponse.json(
        { error: `Jupiter price failed (${res.status})` },
        { status: res.status },
      );
    }
    return NextResponse.json(data);
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Price request failed" },
      { status: 502 },
    );
  }
}
