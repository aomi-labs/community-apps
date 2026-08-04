"use client";

import * as React from "react";
import {
  DEFAULT_ENVELOPE,
  evaluate,
  ProposedAction,
  Verdict,
} from "@/lib/envelope";
import { USDC, SOL, LOOKALIKE_SOL, toBaseUnits, fromBaseUnits } from "@/lib/tokens";
import { parseRule, ParsedRule, EXAMPLE_RULES } from "@/lib/rule";
import { QuoteResponse, fetchQuote, routeLabels } from "@/lib/jupiter";
import { usd, num, pct, shortMint, clockLabel, timeUntilNextDaily } from "@/lib/format";
import { Panel, PanelHeader, Badge, Button, LiveDot, Row, cn } from "@/components/ui";
import { DifferentPanel } from "@/components/DifferentPanel";

const ENV = DEFAULT_ENVELOPE;
const VENUE = "Jupiter aggregator";

type StagedEntry = {
  id: string;
  ts: string;
  title: string;
  action: ProposedAction;
  verdict: Verdict;
  quote?: QuoteResponse;
  status: "staged" | "blocked" | "signed";
  note?: string;
};

function actionFor(amountUsd: number, output = SOL, input = USDC): ProposedAction {
  return {
    inputToken: input,
    outputToken: output,
    amountUsd,
    venue: VENUE,
    slippageBps: ENV.maxSlippageBps,
  };
}

// Seeds are computed through the REAL evaluate() — the verdicts are not
// hardcoded. Timestamps are static strings so server and client render alike.
function seedLog(): StagedEntry[] {
  const oversize = actionFor(500);
  const lookalike = actionFor(20, LOOKALIKE_SOL);
  return [
    {
      id: "seed-oversize",
      ts: "02:14",
      title: "Buy $500 of SOL",
      action: oversize,
      verdict: evaluate(oversize, ENV),
      status: "blocked",
      note: "A larger buy was proposed. It never reached a quote.",
    },
    {
      id: "seed-lookalike",
      ts: "01:47",
      title: "Swap $20 USDC → “SOL” (unverified mint)",
      action: lookalike,
      verdict: evaluate(lookalike, ENV),
      status: "blocked",
      note: "A token sharing the SOL ticker but not the canonical mint.",
    },
  ];
}

export default function NightshiftApp() {
  const [mounted, setMounted] = React.useState(false);
  const [now, setNow] = React.useState(() => new Date(0));
  const [ruleText, setRuleText] = React.useState(EXAMPLE_RULES[0]);
  const [rule, setRule] = React.useState<ParsedRule>(() => parseRule(EXAMPLE_RULES[0]));
  const [solPrice, setSolPrice] = React.useState<number | null>(null);

  const [quote, setQuote] = React.useState<QuoteResponse | null>(null);
  const [quoteErr, setQuoteErr] = React.useState<string | null>(null);
  const [quoting, setQuoting] = React.useState(false);

  const [log, setLog] = React.useState<StagedEntry[]>(seedLog);

  React.useEffect(() => {
    setMounted(true);
    setNow(new Date());
    const t = setInterval(() => setNow(new Date()), 1000);
    return () => clearInterval(t);
  }, []);

  // Live SOL price.
  React.useEffect(() => {
    let live = true;
    const load = async () => {
      try {
        const res = await fetch(`/api/price?ids=${SOL.mint}`, { cache: "no-store" });
        const data = await res.json();
        const p = data?.[SOL.mint]?.usdPrice;
        if (live && typeof p === "number") setSolPrice(p);
      } catch {
        /* keep last known */
      }
    };
    load();
    const t = setInterval(load, 20_000);
    return () => {
      live = false;
      clearInterval(t);
    };
  }, []);

  const currentAction = rule.ok ? actionFor(rule.amountUsd, rule.outputToken, rule.inputToken) : null;
  const currentVerdict = currentAction ? evaluate(currentAction, ENV) : null;

  // Fetch a live quote for the next action whenever the rule changes.
  const refreshQuote = React.useCallback(async () => {
    if (!rule.ok || !currentVerdict?.allowed) {
      setQuote(null);
      setQuoteErr(null);
      return;
    }
    setQuoting(true);
    setQuoteErr(null);
    const amount = toBaseUnits(rule.amountUsd, rule.inputToken.decimals);
    const r = await fetchQuote({
      inputMint: rule.inputToken.mint,
      outputMint: rule.outputToken.mint,
      amount,
      slippageBps: ENV.maxSlippageBps,
    });
    if (r.ok) {
      setQuote(r.quote);
    } else {
      setQuote(null);
      setQuoteErr(r.error);
    }
    setQuoting(false);
  }, [rule, currentVerdict?.allowed]);

  React.useEffect(() => {
    refreshQuote();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [rule.summary]);

  const applyRule = (text: string) => {
    setRuleText(text);
    setRule(parseRule(text));
  };

  const outUi = quote ? fromBaseUnits(quote.outAmount, SOL.decimals) : 0;
  const minUi = quote ? fromBaseUnits(quote.otherAmountThreshold, SOL.decimals) : 0;
  const impact = quote ? Number(quote.priceImpactPct) : 0;

  const stageCurrent = () => {
    if (!currentAction || !currentVerdict?.allowed || !quote) return;
    const entry: StagedEntry = {
      id: `staged-${log.length}-${quote.contextSlot ?? outUi}`,
      ts: mounted ? clockLabel(new Date()) : "—",
      title: rule.summary,
      action: currentAction,
      verdict: currentVerdict,
      quote,
      status: "staged",
      note: "Simulated and staged inside the envelope. Not signed.",
    };
    setLog((prev) => [entry, ...prev]);
  };

  const markSigned = (id: string, sig?: string) => {
    setLog((prev) =>
      prev.map((e) =>
        e.id === id
          ? {
              ...e,
              status: "signed",
              note: sig ? `Broadcast · ${shortMint(sig)}` : "Signed by wallet. Not broadcast.",
            }
          : e,
      ),
    );
  };

  const topStaged = log.find((e) => e.status === "staged");

  return (
    <div className="min-h-screen">
      <Header now={now} mounted={mounted} solPrice={solPrice} />

      <main className="mx-auto max-w-[1180px] px-5 pb-24 md:px-8">
        {/* Hero line */}
        <div className="pt-10 pb-8 max-w-3xl">
          <div
            className="font-mono text-[11px] uppercase mb-3"
            style={{ letterSpacing: "var(--aomi-tracking-wide)", color: "var(--aomi-accent)" }}
          >
            Nightshift · a bounded Solana agent
          </div>
          <h1
            className="font-display text-3xl md:text-[2.6rem] leading-[1.1]"
            style={{ color: "var(--aomi-text)", letterSpacing: "var(--aomi-tracking-tight)" }}
          >
            An agent that trades while you sleep, inside a box you can read.
          </h1>
          <p className="mt-4 text-base leading-relaxed" style={{ color: "var(--aomi-text-muted)" }}>
            You set one rule in plain English. The agent proposes an action, a live route is
            quoted, and every step is checked against a permission envelope fixed in code. It can
            quote and stage. It cannot color outside the box.
          </p>
          <p className="mt-3 text-sm leading-relaxed" style={{ color: "var(--aomi-text-subtle)" }}>
            A chat that trades needs you present to approve each transaction. Nightshift is the other
            half: a bound you read and grant once, so an agent can act on a schedule while you are
            not watching, and still never step outside the box you approved.
          </p>
        </div>

        <DifferentPanel />

        <div className="grid grid-cols-1 lg:grid-cols-[420px_1fr] gap-5 items-start">
          {/* LEFT: the envelope (hero) */}
          <div className="flex flex-col gap-5 lg:sticky lg:top-5">
            <EnvelopePanel currentVerdict={currentVerdict} />
            <SigningPanel />
            <CannotDoPanel />
          </div>

          {/* RIGHT: rule, next action, log */}
          <div className="flex flex-col gap-5">
            <RuleComposer
              ruleText={ruleText}
              setRuleText={setRuleText}
              onApply={() => applyRule(ruleText)}
              onExample={applyRule}
              rule={rule}
            />

            <NextActionCard
              rule={rule}
              verdict={currentVerdict}
              quote={quote}
              quoting={quoting}
              quoteErr={quoteErr}
              outUi={outUi}
              minUi={minUi}
              impact={impact}
              solPrice={solPrice}
              onRefresh={refreshQuote}
              onStage={stageCurrent}
              canStage={!!(currentVerdict?.allowed && quote)}
              mounted={mounted}
              nextIn={mounted ? timeUntilNextDaily(now) : "—"}
            />

            <SignPanel entry={topStaged} onSigned={markSigned} solPrice={solPrice} />

            <StagedLog log={log} />

            <AutonomousPath />
          </div>
        </div>

        <Footer />
      </main>
    </div>
  );
}

/* ---------------------------------------------------------------- Header */

function Header({
  now,
  mounted,
  solPrice,
}: {
  now: Date;
  mounted: boolean;
  solPrice: number | null;
}) {
  return (
    <header
      className="sticky top-0 z-20 border-b backdrop-blur"
      style={{
        borderColor: "var(--aomi-hairline)",
        background: "color-mix(in srgb, var(--aomi-bg) 82%, transparent)",
      }}
    >
      <div className="mx-auto max-w-[1180px] px-5 md:px-8 h-14 flex items-center justify-between">
        <div className="flex items-center gap-2.5">
          {/* eslint-disable-next-line @next/next/no-img-element */}
          <img src="/aomi-mark.svg" alt="Aomi" width={22} height={22} style={{ display: "block" }} />
          <span
            className="font-wordmark text-lg"
            style={{ color: "var(--aomi-text)", fontWeight: 600 }}
          >
            aomi
          </span>
          <span
            className="ml-1 font-mono text-[11px] px-2 py-0.5 rounded-pill"
            style={{
              color: "var(--aomi-text-muted)",
              border: "1px solid var(--aomi-border)",
            }}
          >
            nightshift
          </span>
        </div>
        <div className="flex items-center gap-4">
          <div className="hidden sm:flex items-center gap-2">
            <span className="font-mono text-xs" style={{ color: "var(--aomi-text-muted)" }}>
              SOL
            </span>
            <span className="font-mono text-xs" style={{ color: "var(--aomi-text)" }}>
              {solPrice ? usd(solPrice) : "—"}
            </span>
            <LiveDot tone="accent" />
          </div>
          <span className="font-mono text-xs tabular-nums" style={{ color: "var(--aomi-text-muted)" }}>
            {mounted ? clockLabel(now) : "--:--:--"}
          </span>
        </div>
      </div>
    </header>
  );
}

/* -------------------------------------------------------- Envelope panel */

function EnvelopePanel({ currentVerdict }: { currentVerdict: Verdict | null }) {
  const granted = ["svm-reads", "svm-tx-broadcast"];
  return (
    <Panel raised className="overflow-hidden">
      <PanelHeader
        eyebrow="Permission envelope · fixed at build time"
        title="What it is allowed to do"
        right={<Badge tone="accent" mono>ENFORCED</Badge>}
      />
      <div className="px-5 py-4 flex flex-col gap-4">
        {/* Namespaces — the real manifest of the nightshift plugin */}
        <div>
          <FieldLabel>Namespace permissions</FieldLabel>
          <div className="flex flex-wrap gap-2 mt-2">
            {granted.map((n) => (
              <Badge key={n} tone="accent" mono>
                {n}
              </Badge>
            ))}
          </div>
          <p className="mt-2 text-xs leading-relaxed" style={{ color: "var(--aomi-text-subtle)" }}>
            These two are the whole manifest. It can read routes and stage a swap for your wallet.
            It holds no key: the wallet signs and broadcasts. Powers it was not compiled with are
            not available at run time, whatever it is asked.
          </p>
        </div>

        <Divider />

        {/* Allowed tokens */}
        <div>
          <FieldLabel>Tokens it may touch</FieldLabel>
          <div className="flex flex-col gap-2 mt-2">
            {ENV.allowedTokens.map((t) => (
              <div
                key={t.mint}
                className="flex items-center justify-between rounded-lg border px-3 py-2"
                style={{ borderColor: "var(--aomi-hairline)", background: "var(--aomi-surface)" }}
              >
                <div className="flex items-center gap-2.5">
                  <span
                    className="font-display text-sm"
                    style={{ color: "var(--aomi-text)" }}
                  >
                    {t.symbol}
                  </span>
                  <span className="font-mono text-[11px]" style={{ color: "var(--aomi-text-subtle)" }}>
                    {shortMint(t.mint)}
                  </span>
                </div>
                <Badge tone="success" mono>
                  verified mint
                </Badge>
              </div>
            ))}
          </div>
          <p className="mt-2 text-xs leading-relaxed" style={{ color: "var(--aomi-text-subtle)" }}>
            Allowlisted by mint, not by ticker. A token that merely calls itself SOL is a different
            address, and a different address is refused.
          </p>
        </div>

        <Divider />

        {/* Numeric bounds */}
        <div className="flex flex-col">
          <FieldLabel>Hard bounds</FieldLabel>
          <div className="mt-1">
            <Row label="Max per action" value={usd(ENV.maxPerActionUsd)} mono />
            <Row label="Max per day" value={usd(ENV.maxDailyUsd)} mono />
            <Row label="Venue" value={ENV.allowedVenues.join(", ")} />
            <Row label="Max slippage" value={`${ENV.maxSlippageBps} bps (${(ENV.maxSlippageBps / 100).toFixed(2)}%)`} mono />
          </div>
        </div>

        <div
          className="rounded-lg px-3 py-3 text-xs leading-relaxed"
          style={{
            background: "var(--aomi-accent-subtle)",
            color: "var(--aomi-accent)",
            border: "1px solid color-mix(in srgb, var(--aomi-accent) 30%, transparent)",
          }}
        >
          The agent proposes. The boundary decides. The boundary is set here, in code — not by the
          model, and not at run time.
          {currentVerdict && (
            <span
              className="ml-1"
              style={{ color: currentVerdict.allowed ? "var(--aomi-success)" : "var(--aomi-danger)" }}
            >
              {currentVerdict.allowed
                ? "Your current rule sits inside it."
                : "Your current rule would be refused."}
            </span>
          )}
        </div>
      </div>
    </Panel>
  );
}

function CannotDoPanel() {
  const cannot = [
    "Touch any token beyond USDC and SOL",
    `Move more than ${usd(ENV.maxPerActionUsd)} in one action`,
    `Spend more than ${usd(ENV.maxDailyUsd)} across the day`,
    "Route through any venue but Jupiter",
    "Sign or move a lamport without your wallet",
  ];
  return (
    <Panel className="overflow-hidden">
      <PanelHeader eyebrow="The short version" title="What it cannot do tonight" />
      <div className="px-5 py-4 flex flex-col gap-2">
        {cannot.map((c) => (
          <div key={c} className="flex items-start gap-2.5">
            <span
              className="mt-[3px] shrink-0"
              style={{ color: "var(--aomi-danger)", fontSize: "13px", lineHeight: 1 }}
            >
              ✕
            </span>
            <span className="text-sm leading-snug" style={{ color: "var(--aomi-text-secondary)" }}>
              {c}
            </span>
          </div>
        ))}
      </div>
    </Panel>
  );
}

function SigningPanel() {
  return (
    <Panel className="overflow-hidden">
      <PanelHeader
        eyebrow="Kernel policy · two decisions the agent never makes"
        title="Who signs, who submits"
      />
      <div className="px-5 py-4 flex flex-col gap-3">
        <PolicyRow
          k="Who signs"
          options={["your wallet", "a delegated grant", "denied"]}
          note="A grant is a scoped permission you hand out, bounded by this same envelope. Absent a valid path, the answer is denied."
        />
        <Divider />
        <PolicyRow
          k="Who submits"
          options={["your wallet", "the venue", "aomi"]}
          note="Routing the signed transaction on-chain is separate from authoring it. The model touches neither decision."
        />
      </div>
    </Panel>
  );
}

function PolicyRow({ k, options, note }: { k: string; options: string[]; note: string }) {
  return (
    <div>
      <div className="flex items-center justify-between">
        <span className="text-sm" style={{ color: "var(--aomi-text-muted)" }}>
          {k}
        </span>
        <div className="flex items-center gap-1.5">
          {options.map((o, i) => (
            <React.Fragment key={o}>
              <span
                className="font-mono text-[11px]"
                style={{
                  color:
                    o === "denied"
                      ? "var(--aomi-danger)"
                      : i === 0
                        ? "var(--aomi-text)"
                        : "var(--aomi-text-muted)",
                }}
              >
                {o}
              </span>
              {i < options.length - 1 && (
                <span style={{ color: "var(--aomi-text-subtle)" }}>·</span>
              )}
            </React.Fragment>
          ))}
        </div>
      </div>
      <p className="mt-1.5 text-xs leading-relaxed" style={{ color: "var(--aomi-text-subtle)" }}>
        {note}
      </p>
    </div>
  );
}

/* --------------------------------------------------------- Rule composer */

function RuleComposer({
  ruleText,
  setRuleText,
  onApply,
  onExample,
  rule,
}: {
  ruleText: string;
  setRuleText: (s: string) => void;
  onApply: () => void;
  onExample: (s: string) => void;
  rule: ParsedRule;
}) {
  return (
    <Panel className="overflow-hidden">
      <PanelHeader eyebrow="Step 1 · your rule" title="Say it in plain English" />
      <div className="px-5 py-4">
        <div className="flex flex-col sm:flex-row gap-2.5">
          <input
            value={ruleText}
            onChange={(e) => setRuleText(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && onApply()}
            spellCheck={false}
            className="flex-1 h-11 rounded-pill px-4 text-sm outline-none font-sans"
            style={{
              background: "var(--aomi-bg-subtle)",
              color: "var(--aomi-text)",
              border: "1px solid var(--aomi-border)",
            }}
            placeholder="Every day, put $20 into SOL"
          />
          <Button variant="primary" onClick={onApply}>
            Set rule
          </Button>
        </div>

        <div className="flex flex-wrap gap-2 mt-3">
          {EXAMPLE_RULES.map((ex) => (
            <button
              key={ex}
              onClick={() => onExample(ex)}
              className="rounded-pill px-3 py-1.5 text-xs transition-colors"
              style={{
                border: "1px solid var(--aomi-border)",
                color: "var(--aomi-text-muted)",
                background: ruleText === ex ? "var(--aomi-accent-subtle)" : "transparent",
              }}
            >
              {ex}
            </button>
          ))}
        </div>

        <div className="mt-4 flex items-start gap-2.5">
          <span
            className="mt-0.5 font-mono text-[10px] px-2 py-0.5 rounded"
            style={{ background: "var(--aomi-surface-2)", color: "var(--aomi-text-muted)" }}
          >
            PARSED
          </span>
          {rule.ok ? (
            <p className="text-sm leading-snug" style={{ color: "var(--aomi-text)" }}>
              {rule.summary}
              <span className="block font-mono text-[11px] mt-1" style={{ color: "var(--aomi-text-subtle)" }}>
                {rule.trigger.kind === "price"
                  ? `trigger: price · ${rule.trigger.label}`
                  : `trigger: schedule · ${rule.trigger.label}`}
              </span>
            </p>
          ) : (
            <p className="text-sm leading-snug" style={{ color: "var(--aomi-warning)" }}>
              {rule.reason}
            </p>
          )}
        </div>
        <p className="mt-3 text-xs leading-relaxed" style={{ color: "var(--aomi-text-subtle)" }}>
          A small deterministic parser reads your rule into a structured action. It is not a model,
          and it refuses shapes it does not understand rather than guessing.
        </p>
      </div>
    </Panel>
  );
}

/* ------------------------------------------------------- Next action card */

function NextActionCard({
  rule,
  verdict,
  quote,
  quoting,
  quoteErr,
  outUi,
  minUi,
  impact,
  solPrice,
  onRefresh,
  onStage,
  canStage,
  mounted,
  nextIn,
}: {
  rule: ParsedRule;
  verdict: Verdict | null;
  quote: QuoteResponse | null;
  quoting: boolean;
  quoteErr: string | null;
  outUi: number;
  minUi: number;
  impact: number;
  solPrice: number | null;
  onRefresh: () => void;
  onStage: () => void;
  canStage: boolean;
  mounted: boolean;
  nextIn: string;
}) {
  const triggerReady =
    rule.ok && rule.trigger.kind === "price" && solPrice != null
      ? rule.trigger.direction === "below"
        ? solPrice < rule.trigger.usd
        : solPrice > rule.trigger.usd
      : true;

  return (
    <Panel className="overflow-hidden">
      <PanelHeader
        eyebrow="Step 2 · next planned action"
        title="What it would do right now"
        right={
          <div className="flex items-center gap-2">
            <LiveDot tone="accent" />
            <span className="font-mono text-[11px]" style={{ color: "var(--aomi-text-muted)" }}>
              live quote
            </span>
          </div>
        }
      />

      <div className="px-5 py-4">
        {/* schedule / trigger strip */}
        <div
          className="flex flex-wrap items-center gap-x-5 gap-y-1 rounded-lg px-3 py-2.5 mb-4"
          style={{ background: "var(--aomi-bg-subtle)", border: "1px solid var(--aomi-hairline)" }}
        >
          {rule.ok && rule.trigger.kind === "schedule" ? (
            <>
              <MiniStat k="cadence" v={rule.trigger.label} />
              <MiniStat k="next run" v={mounted ? `in ${nextIn}` : "—"} />
            </>
          ) : rule.ok && rule.trigger.kind === "price" ? (
            <>
              <MiniStat k="trigger" v={rule.trigger.label} />
              <MiniStat
                k="status"
                v={triggerReady ? "condition met" : "waiting"}
                tone={triggerReady ? "success" : "muted"}
              />
            </>
          ) : (
            <MiniStat k="rule" v="not set" tone="muted" />
          )}
        </div>

        {!rule.ok ? (
          <Empty>Set a rule above to see the next action and a live route.</Empty>
        ) : !verdict?.allowed ? (
          <div
            className="rounded-lg px-4 py-4 text-sm"
            style={{
              background: "color-mix(in srgb, var(--aomi-danger) 8%, transparent)",
              border: "1px solid color-mix(in srgb, var(--aomi-danger) 30%, transparent)",
              color: "var(--aomi-danger)",
            }}
          >
            This action is outside the envelope, so it is never quoted or staged.
            <ul className="mt-2 space-y-1">
              {verdict?.violations.map((v, i) => (
                <li key={i} className="font-mono text-[11px]">
                  ✕ {v.rule}: {v.detail}
                </li>
              ))}
            </ul>
          </div>
        ) : (
          <>
            {/* the swap, drawn */}
            <div className="flex items-stretch gap-3">
              <Leg
                top={usd(rule.amountUsd)}
                sym={rule.inputToken.symbol}
                sub="you spend"
              />
              <div className="flex flex-col items-center justify-center px-1">
                <span style={{ color: "var(--aomi-text-subtle)" }}>→</span>
              </div>
              <Leg
                top={quote ? num(outUi, 5) : quoting ? "…" : "—"}
                sym={rule.outputToken.symbol}
                sub="you receive"
                accent
              />
            </div>

            <div className="mt-4">
              <Row
                label="Route"
                value={quote ? routeLabels(quote) : quoting ? "quoting…" : "—"}
                mono
              />
              <Row
                label="Price impact"
                value={quote ? pct(impact) : "—"}
                mono
                tone={impact > 0.01 ? "default" : "muted"}
              />
              <Row
                label="Min received (after slippage)"
                value={quote ? `${num(minUi, 5)} SOL` : "—"}
                mono
              />
              <Row
                label="Quoted value"
                value={quote?.swapUsdValue ? usd(Number(quote.swapUsdValue)) : "—"}
                mono
              />
              <Row
                label="Envelope check"
                value={
                  <span style={{ color: "var(--aomi-success)" }}>inside the envelope</span>
                }
              />
            </div>

            {quoteErr && (
              <p className="mt-3 text-xs" style={{ color: "var(--aomi-warning)" }}>
                {quoteErr}
              </p>
            )}

            <div className="flex items-center gap-2.5 mt-4">
              <Button variant="accent" onClick={onStage} disabled={!canStage}>
                Stage this action
              </Button>
              <Button variant="outline" onClick={onRefresh} disabled={quoting}>
                {quoting ? "Refreshing…" : "Refresh quote"}
              </Button>
            </div>
            <p className="mt-3 text-xs leading-relaxed" style={{ color: "var(--aomi-text-subtle)" }}>
              The route, price impact and minimum received are a live Jupiter quote. The scheduler
              itself is simulated for this demo — the numbers are not.
            </p>
          </>
        )}
      </div>
    </Panel>
  );
}

/* ----------------------------------------------------------- Sign panel */

type Phantom = {
  isPhantom?: boolean;
  publicKey?: { toString(): string };
  connect: () => Promise<{ publicKey: { toString(): string } }>;
  signTransaction: (tx: unknown) => Promise<unknown>;
};

function getPhantom(): Phantom | null {
  if (typeof window === "undefined") return null;
  const w = window as unknown as { solana?: Phantom; phantom?: { solana?: Phantom } };
  const p = w.phantom?.solana ?? w.solana;
  return p?.isPhantom ? p : null;
}

function SignPanel({
  entry,
  onSigned,
  solPrice,
}: {
  entry?: StagedEntry;
  onSigned: (id: string, sig?: string) => void;
  solPrice: number | null;
}) {
  const [status, setStatus] = React.useState<string>("");
  const [busy, setBusy] = React.useState(false);
  const [pubkey, setPubkey] = React.useState<string | null>(null);
  const [signedB64, setSignedB64] = React.useState<string | null>(null);
  const [err, setErr] = React.useState<string | null>(null);

  const hasPhantom = typeof window !== "undefined" && !!getPhantom();

  const sign = async () => {
    setErr(null);
    setSignedB64(null);
    if (!entry?.quote) return;
    const provider = getPhantom();
    if (!provider) {
      setErr("Phantom not detected. Install it, or use the autonomous path below.");
      return;
    }
    setBusy(true);
    try {
      setStatus("Connecting Phantom…");
      const { publicKey } = await provider.connect();
      const pk = publicKey.toString();
      setPubkey(pk);

      setStatus("Building the real swap transaction…");
      const res = await fetch("/api/swap", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ quoteResponse: entry.quote, userPublicKey: pk }),
      });
      const data = await res.json();
      if (!res.ok || data.error) throw new Error(data.error ?? "Could not build the swap");

      setStatus("Waiting for your signature in Phantom…");
      const { VersionedTransaction } = await import("@solana/web3.js");
      const buf = Uint8Array.from(atob(data.swapTransaction), (c) => c.charCodeAt(0));
      const tx = VersionedTransaction.deserialize(buf);
      const signed = (await provider.signTransaction(tx)) as {
        serialize: () => Uint8Array;
      };
      const out = signed.serialize();
      let bin = "";
      out.forEach((b) => (bin += String.fromCharCode(b)));
      setSignedB64(btoa(bin));
      setStatus("Signed. This exact transaction is what the autonomous path would sign too.");
      onSigned(entry.id);
    } catch (e) {
      setErr(e instanceof Error ? e.message : "Signing was cancelled");
      setStatus("");
    } finally {
      setBusy(false);
    }
  };

  const broadcast = async () => {
    if (!signedB64 || !entry) return;
    setErr(null);
    setBusy(true);
    try {
      setStatus("Broadcasting to Solana mainnet…");
      const res = await fetch("/api/broadcast", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ signedTransaction: signedB64 }),
      });
      const data = await res.json();
      if (!res.ok || data.error) throw new Error(data.error ?? "Broadcast rejected");
      setStatus(`Confirmed on-chain · ${shortMint(data.signature)}`);
      onSigned(entry.id, data.signature);
      setSignedB64(null);
    } catch (e) {
      setErr(e instanceof Error ? e.message : "Broadcast failed");
    } finally {
      setBusy(false);
    }
  };

  return (
    <Panel className="overflow-hidden">
      <PanelHeader
        eyebrow="Step 3 · sign one for real"
        title="You hold the pen"
        right={
          hasPhantom ? (
            <Badge tone="success" mono>phantom ready</Badge>
          ) : (
            <Badge tone="neutral" mono>phantom not found</Badge>
          )
        }
      />
      <div className="px-5 py-4">
        {!entry ? (
          <Empty>Stage an action above, then sign it here.</Empty>
        ) : (
          <>
            <div
              className="rounded-lg px-3 py-2.5 mb-3 text-sm"
              style={{ background: "var(--aomi-bg-subtle)", border: "1px solid var(--aomi-hairline)" }}
            >
              <span style={{ color: "var(--aomi-text)" }}>{entry.title}</span>
              <span className="block font-mono text-[11px] mt-1" style={{ color: "var(--aomi-text-subtle)" }}>
                staged {entry.ts} · inside the envelope
              </span>
            </div>

            <div className="flex flex-wrap items-center gap-2.5">
              <Button variant="primary" onClick={sign} disabled={busy}>
                {busy ? "Working…" : "Sign the next one now"}
              </Button>
              {signedB64 && (
                <Button variant="outline" onClick={broadcast} disabled={busy}>
                  Broadcast (spends real funds)
                </Button>
              )}
              {pubkey && (
                <span className="font-mono text-[11px]" style={{ color: "var(--aomi-text-muted)" }}>
                  {shortMint(pubkey)}
                </span>
              )}
            </div>

            {status && (
              <p className="mt-3 text-xs" style={{ color: "var(--aomi-accent)" }}>
                {status}
              </p>
            )}
            {err && (
              <p className="mt-2 text-xs" style={{ color: "var(--aomi-warning)" }}>
                {err}
              </p>
            )}
            <p className="mt-3 text-xs leading-relaxed" style={{ color: "var(--aomi-text-subtle)" }}>
              Signing builds and signs the real Jupiter swap for the staged action. It stops at a
              signature — broadcasting is a second, deliberate click, because it is your money.
            </p>
          </>
        )}
      </div>
    </Panel>
  );
}

/* ------------------------------------------------------------ Staged log */

function StagedLog({ log }: { log: StagedEntry[] }) {
  return (
    <Panel className="overflow-hidden">
      <PanelHeader
        eyebrow="Ledger · every proposal, kept"
        title="Staged actions"
        right={
          <span className="font-mono text-[11px]" style={{ color: "var(--aomi-text-muted)" }}>
            {log.length} entries
          </span>
        }
      />
      <div className="flex flex-col">
        {log.map((e) => (
          <LogItem key={e.id} e={e} />
        ))}
      </div>
    </Panel>
  );
}

function LogItem({ e }: { e: StagedEntry }) {
  const blocked = e.status === "blocked";
  const signed = e.status === "signed";
  return (
    <div
      className="px-5 py-3.5 border-b last:border-b-0 rise"
      style={{ borderColor: "var(--aomi-hairline)" }}
    >
      <div className="flex items-start justify-between gap-4">
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            <span
              className="font-mono text-[11px] tabular-nums"
              style={{ color: "var(--aomi-text-subtle)" }}
            >
              {e.ts}
            </span>
            <span className="text-sm truncate" style={{ color: "var(--aomi-text)" }}>
              {e.title}
            </span>
          </div>
          {e.note && (
            <p className="mt-1 text-xs" style={{ color: "var(--aomi-text-subtle)" }}>
              {e.note}
            </p>
          )}
          {/* simulate-before-sign preview for staged/signed */}
          {e.quote && (
            <div className="mt-2 flex flex-wrap gap-x-4 gap-y-1">
              <Chip
                k="route"
                v={routeLabels(e.quote)}
              />
              <Chip k="impact" v={pct(Number(e.quote.priceImpactPct))} />
              <Chip
                k="min out"
                v={`${num(fromBaseUnits(e.quote.otherAmountThreshold, SOL.decimals), 4)} SOL`}
              />
            </div>
          )}
          {/* violations for blocked */}
          {blocked && (
            <div className="mt-2 flex flex-col gap-1">
              {e.verdict.violations.map((v, i) => (
                <span key={i} className="font-mono text-[11px]" style={{ color: "var(--aomi-danger)" }}>
                  ✕ {v.rule}: {v.detail}
                </span>
              ))}
            </div>
          )}
        </div>
        <div className="shrink-0">
          {blocked ? (
            <Badge tone="danger" mono>blocked</Badge>
          ) : signed ? (
            <Badge tone="success" mono>signed</Badge>
          ) : (
            <Badge tone="accent" mono>inside envelope</Badge>
          )}
        </div>
      </div>
    </div>
  );
}

/* ------------------------------------------------------- Autonomous path */

function AutonomousPath() {
  return (
    <Panel className="overflow-hidden">
      <PanelHeader
        eyebrow="The other signer · same box"
        title="How the autonomous path signs"
      />
      <div className="px-5 py-4">
        <ol className="flex flex-col gap-3">
          {[
            ["Trigger fires", "The schedule or price condition is met. The agent drafts the same action you saw above."],
            ["Envelope check", "The identical evaluate() runs. Outside the box, it never reaches a signer — exactly as the blocked entries never reached a quote."],
            ["Delegated grant signs", "Server-side, a scoped grant signs the staged transaction. The grant is bounded by this envelope, so it can only ever sign what you could stage."],
            ["Submit", "The signed transaction is routed on-chain. You wake up to a ledger entry, not a surprise."],
          ].map(([k, v], i) => (
            <li key={k} className="flex gap-3">
              <span
                className="shrink-0 h-6 w-6 rounded-full grid place-items-center font-mono text-[11px]"
                style={{ background: "var(--aomi-surface-2)", color: "var(--aomi-text-muted)" }}
              >
                {i + 1}
              </span>
              <div>
                <span className="text-sm" style={{ color: "var(--aomi-text)" }}>{k}</span>
                <p className="text-xs leading-relaxed mt-0.5" style={{ color: "var(--aomi-text-muted)" }}>{v}</p>
              </div>
            </li>
          ))}
        </ol>
        <div
          className="mt-4 rounded-lg px-3 py-2.5 text-xs leading-relaxed"
          style={{
            background: "color-mix(in srgb, var(--aomi-warning) 10%, transparent)",
            border: "1px solid color-mix(in srgb, var(--aomi-warning) 28%, transparent)",
            color: "var(--aomi-warning)",
          }}
        >
          In this demo the autonomous signer is described, not run — the live signing path here is
          Phantom, above. What is real either way: the envelope both signers must pass is the same
          code.
        </div>
      </div>
    </Panel>
  );
}

/* --------------------------------------------------------------- Bits */

function FieldLabel({ children }: { children: React.ReactNode }) {
  return (
    <span
      className="font-mono text-[10.5px] uppercase"
      style={{ letterSpacing: "var(--aomi-tracking-wide)", color: "var(--aomi-text-subtle)" }}
    >
      {children}
    </span>
  );
}

function Divider() {
  return <div className="h-px" style={{ background: "var(--aomi-hairline)" }} />;
}

function MiniStat({ k, v, tone }: { k: string; v: string; tone?: "muted" | "success" }) {
  return (
    <div className="flex items-baseline gap-1.5">
      <span className="font-mono text-[10px] uppercase" style={{ color: "var(--aomi-text-subtle)", letterSpacing: "0.04em" }}>
        {k}
      </span>
      <span
        className="text-sm"
        style={{ color: tone === "success" ? "var(--aomi-success)" : tone === "muted" ? "var(--aomi-text-muted)" : "var(--aomi-text)" }}
      >
        {v}
      </span>
    </div>
  );
}

function Leg({ top, sym, sub, accent }: { top: string; sym: string; sub: string; accent?: boolean }) {
  return (
    <div
      className="flex-1 rounded-xl px-4 py-3.5"
      style={{
        background: accent ? "var(--aomi-accent-subtle)" : "var(--aomi-bg-subtle)",
        border: `1px solid ${accent ? "color-mix(in srgb, var(--aomi-accent) 30%, transparent)" : "var(--aomi-hairline)"}`,
      }}
    >
      <div className="font-mono text-[10px] uppercase" style={{ color: "var(--aomi-text-subtle)", letterSpacing: "0.04em" }}>
        {sub}
      </div>
      <div className="mt-1 flex items-baseline gap-1.5">
        <span className="font-display text-xl tabular-nums" style={{ color: accent ? "var(--aomi-accent)" : "var(--aomi-text)" }}>
          {top}
        </span>
        <span className="text-sm" style={{ color: "var(--aomi-text-muted)" }}>{sym}</span>
      </div>
    </div>
  );
}

function Chip({ k, v }: { k: string; v: string }) {
  return (
    <span className="font-mono text-[11px]" style={{ color: "var(--aomi-text-muted)" }}>
      <span style={{ color: "var(--aomi-text-subtle)" }}>{k} </span>
      {v}
    </span>
  );
}

function Empty({ children }: { children: React.ReactNode }) {
  return (
    <div
      className="rounded-lg px-4 py-6 text-center text-sm"
      style={{ background: "var(--aomi-bg-subtle)", color: "var(--aomi-text-muted)", border: "1px dashed var(--aomi-border)" }}
    >
      {children}
    </div>
  );
}

function Footer() {
  return (
    <footer className="mt-12 pt-6 border-t" style={{ borderColor: "var(--aomi-hairline)" }}>
      <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3">
        <p className="text-xs leading-relaxed max-w-lg" style={{ color: "var(--aomi-text-subtle)" }}>
          Nightshift is a demo of one idea: an agent whose powers are fixed in code, legible before
          you let it run. Quotes and mint checks are live from Jupiter. The scheduler and the
          autonomous signer are simulated and labeled as such. No keys, no custody.
        </p>
        <div className="flex items-center gap-2.5">
          <a
            href="https://aomi.dev/docs/build"
            target="_blank"
            rel="noreferrer"
            className="font-mono text-[11px] px-3 py-1.5 rounded-pill"
            style={{ border: "1px solid var(--aomi-border)", color: "var(--aomi-text-muted)" }}
          >
            aomi.dev/docs/build
          </a>
        </div>
      </div>
    </footer>
  );
}
