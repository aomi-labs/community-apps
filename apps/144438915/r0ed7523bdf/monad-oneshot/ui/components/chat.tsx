import type { UnsignedTx } from "@/lib/aomi-chat";
import { AomiMark } from "./AomiMark";

export type Bubble =
  | { kind: "user"; text: string }
  | { kind: "agent"; text: string; tx?: UnsignedTx }
  | { kind: "system"; text: string };

const EXPLORER = "https://monadscan.com/tx/";

function shortHash(h: string) {
  return `${h.slice(0, 6)}…${h.slice(-4)}`;
}

/** Embedded preview of the unsigned transaction the agent built. */
function TxCard({ tx }: { tx: UnsignedTx }) {
  return (
    <div className="txcard">
      <div className="txcard-head">
        <span className="t">Unsigned transaction</span>
        <span className="await">
          <span className="p" />
          Awaiting signature
        </span>
      </div>
      <div className="txrows">
        <div className="txrow">
          <span className="k">to</span>
          <span className="v addr">{tx.to}</span>
        </div>
        <div className="txrow">
          <span className="k">value</span>
          <span className="v">{tx.value}</span>
        </div>
        <div className="txrow">
          <span className="k">chainId</span>
          <span className="v">{tx.chainId}</span>
        </div>
      </div>
    </div>
  );
}

/** A single thread item. Text content is set by the page; this only renders it. */
export function ChatMessage({ bubble }: { bubble: Bubble }) {
  if (bubble.kind === "user") {
    return (
      <div className="row user">
        <div className="bubble user">{bubble.text}</div>
      </div>
    );
  }

  if (bubble.kind === "agent") {
    return (
      <div className="row agent">
        <div className="agent-wrap">
          <span className="agent-ava" style={{ color: "var(--pink)" }}>
            <AomiMark />
          </span>
          <div className="bubble agent">
            {bubble.text}
            {bubble.tx && <TxCard tx={bubble.tx} />}
          </div>
        </div>
      </div>
    );
  }

  // system — slimmer, less prominent. Broadcast hashes link to the explorer.
  const broadcast = bubble.text.match(/^Broadcast:\s*(0x[0-9a-fA-F]+)/);
  const isError = /^(Error|Sign failed|Connect failed)/i.test(bubble.text);
  const cls = `sysmsg${broadcast ? " ok" : isError ? " err" : ""}`;

  return (
    <div className="row sys">
      <div className={cls}>
        <span className="sdot" />
        {broadcast ? (
          <span>
            Broadcast ·{" "}
            <a href={`${EXPLORER}${broadcast[1]}`} target="_blank" rel="noopener noreferrer">
              {shortHash(broadcast[1])}
            </a>{" "}
            <span className="arr">↗</span>
          </span>
        ) : (
          <span>{bubble.text}</span>
        )}
      </div>
    </div>
  );
}

/** Inviting empty state, shown until the first user message. */
export function EmptyState() {
  return (
    <div className="empty">
      <AomiMark className="mk" />
      <h3>Quiet here</h3>
      <p>
        Pick a starter above, or just ask — <span className="q">&quot;what is my MON balance?&quot;</span> Aomi
        turns your words into one signed action on Monad.
      </p>
    </div>
  );
}

/** Subtle loading indicator shown above the composer while the agent works. */
export function ThinkingIndicator() {
  return (
    <div className="thinking">
      <span className="dots">
        <i />
        <i />
        <i />
      </span>
      Aomi is thinking
    </div>
  );
}
