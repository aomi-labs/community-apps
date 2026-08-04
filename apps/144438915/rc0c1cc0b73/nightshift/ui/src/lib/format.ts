export function usd(n: number, max = 2): string {
  if (!isFinite(n)) return "—";
  return n.toLocaleString("en-US", {
    style: "currency",
    currency: "USD",
    minimumFractionDigits: n < 1 ? 4 : 2,
    maximumFractionDigits: n < 1 ? 6 : max,
  });
}

export function num(n: number, max = 6): string {
  if (!isFinite(n)) return "—";
  return n.toLocaleString("en-US", { maximumFractionDigits: max });
}

export function pct(fraction: number): string {
  const p = fraction * 100;
  if (Math.abs(p) < 0.0001) return "~0.00%";
  return `${p < 0 ? "" : ""}${p.toFixed(p < 0.01 ? 4 : 2)}%`;
}

export function shortMint(mint: string): string {
  return `${mint.slice(0, 4)}…${mint.slice(-4)}`;
}

export function timeUntilNextDaily(now = new Date()): string {
  // Next 03:00 local — the "nightshift" window.
  const next = new Date(now);
  next.setHours(3, 0, 0, 0);
  if (next.getTime() <= now.getTime()) next.setDate(next.getDate() + 1);
  const ms = next.getTime() - now.getTime();
  const h = Math.floor(ms / 3_600_000);
  const m = Math.floor((ms % 3_600_000) / 60_000);
  return `${h}h ${m}m`;
}

export function clockLabel(d = new Date()): string {
  return d.toLocaleTimeString("en-US", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  });
}
