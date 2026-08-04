import * as React from "react";
import { Panel } from "@/components/ui";

// The answer to "we can already do this in the chat." A chat that trades is
// synchronous: you approve each transaction, in the moment, present. This is
// the other primitive — the boundary itself, rendered so you can read it and
// grant it once. Not a competing surface; the half the chat does not expose.
export function DifferentPanel() {
  const cols = [
    {
      eyebrow: "A chat that trades",
      title: "You approve each transaction",
      points: [
        "Synchronous. You are present for every action.",
        "The box is implicit. You trust it one message at a time.",
        "Great for a trade you are watching happen.",
      ],
      tone: "muted" as const,
    },
    {
      eyebrow: "Nightshift",
      title: "You read the box, then grant it once",
      points: [
        "The full permission surface is one legible spec, up front.",
        "Powers are fixed at build time, so the grant is safe to hand out.",
        "Made for the trade you are asleep for.",
      ],
      tone: "accent" as const,
    },
  ];
  return (
    <Panel raised className="overflow-hidden mb-8">
      <div className="px-5 pt-4 pb-1">
        <div
          className="font-mono text-[10.5px] uppercase"
          style={{ letterSpacing: "var(--aomi-tracking-wide)", color: "var(--aomi-text-subtle)" }}
        >
          Why this is not the chat
        </div>
      </div>
      <div className="grid grid-cols-1 md:grid-cols-2">
        {cols.map((c, i) => (
          <div
            key={c.eyebrow}
            className="px-5 py-4"
            style={{
              borderTop: "1px solid var(--aomi-hairline)",
              borderLeft: i === 1 ? "1px solid var(--aomi-hairline)" : undefined,
            }}
          >
            <div
              className="font-mono text-[10.5px] uppercase mb-1"
              style={{
                letterSpacing: "var(--aomi-tracking-wide)",
                color: c.tone === "accent" ? "var(--aomi-accent)" : "var(--aomi-text-subtle)",
              }}
            >
              {c.eyebrow}
            </div>
            <div
              className="font-display text-base mb-3"
              style={{ color: "var(--aomi-text)" }}
            >
              {c.title}
            </div>
            <ul className="flex flex-col gap-1.5">
              {c.points.map((p) => (
                <li key={p} className="flex items-start gap-2">
                  <span
                    className="mt-[7px] h-1 w-1 rounded-full shrink-0"
                    style={{
                      background:
                        c.tone === "accent" ? "var(--aomi-accent)" : "var(--aomi-text-subtle)",
                    }}
                  />
                  <span className="text-sm leading-snug" style={{ color: "var(--aomi-text-secondary)" }}>
                    {p}
                  </span>
                </li>
              ))}
            </ul>
          </div>
        ))}
      </div>
      <div
        className="px-5 py-3 text-sm leading-relaxed"
        style={{ borderTop: "1px solid var(--aomi-hairline)", color: "var(--aomi-text-muted)" }}
      >
        The chat guards the spend you approve. Nightshift makes the guard itself the product, so an
        agent can act on a schedule inside a box you already read. Same wedge, the other half:
        <span style={{ color: "var(--aomi-text)" }}> they guard the spend, we guard the build.</span>
      </div>
    </Panel>
  );
}
