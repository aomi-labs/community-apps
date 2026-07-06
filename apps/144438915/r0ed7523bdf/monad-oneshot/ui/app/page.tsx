"use client";

import { useEffect, useRef, useState } from "react";
import {
  useAccount,
  useConnect,
  useDisconnect,
  useSendTransaction,
  useSwitchChain,
} from "wagmi";
import { monadMainnet } from "@/lib/chains";
import { sendChat, type ChatMessage, type UnsignedTx } from "@/lib/aomi-chat";
import { BrandLogo, NetworkBadge, AccountButton } from "@/components/header";
import { StarterChips } from "@/components/StarterChips";
import { WrongChainBanner, ErrorBanner } from "@/components/banners";
import {
  ChatMessage as ChatBubble,
  EmptyState,
  ThinkingIndicator,
  type Bubble,
} from "@/components/chat";

const BACKEND_URL =
  process.env.NEXT_PUBLIC_AOMI_BACKEND_URL ?? "http://localhost:8080";
const APP_ID = process.env.NEXT_PUBLIC_AOMI_APP_ID ?? "monad-oneshot";

const EXAMPLES = [
  "What is my MON balance?",
  "Wrap 0.1 MON",
  "Unwrap 0.05 WMON",
  "Send 0.01 MON to 0x000000000000000000000000000000000000dEaD",
];

export default function Page() {
  const { address, isConnected, chainId } = useAccount();
  const { connect, connectors, error: connectError } = useConnect();
  const { disconnect } = useDisconnect();
  const { sendTransactionAsync } = useSendTransaction();
  const { switchChainAsync } = useSwitchChain();

  const onWrongChain = isConnected && chainId !== monadMainnet.id;

  const [bubbles, setBubbles] = useState<Bubble[]>([
    {
      kind: "system",
      text: "Connect a browser wallet (MetaMask, Rabby) on Monad testnet. Then type what you want.",
    },
  ]);
  const [input, setInput] = useState("");
  const [busy, setBusy] = useState(false);
  const endRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    endRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [bubbles]);

  async function submit(message: string) {
    if (!message.trim() || busy) return;
    setBusy(true);
    setBubbles((b) => [...b, { kind: "user", text: message }]);
    setInput("");

    const history: ChatMessage[] = bubbles
      .filter(
        (b): b is Extract<Bubble, { kind: "user" | "agent" }> =>
          b.kind === "user" || b.kind === "agent",
      )
      .map((b) => ({
        role: b.kind === "user" ? "user" : "assistant",
        content: b.text,
      }));

    try {
      const reply = await sendChat({
        backendUrl: BACKEND_URL,
        appId: APP_ID,
        message,
        history,
        userAddress: address,
      });
      setBubbles((b) => [
        ...b,
        {
          kind: "agent",
          text: reply.message || "(no reply)",
          tx: reply.transaction,
        },
      ]);
      if (reply.transaction) await signAndReport(reply.transaction);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setBubbles((b) => [...b, { kind: "system", text: `Error: ${msg}` }]);
    } finally {
      setBusy(false);
    }
  }

  async function signAndReport(tx: UnsignedTx) {
    if (!isConnected) {
      setBubbles((b) => [
        ...b,
        { kind: "system", text: "Connect a wallet before signing." },
      ]);
      return;
    }
    if (chainId !== tx.chainId) {
      try {
        await switchChainAsync({ chainId: tx.chainId });
      } catch (err) {
        setBubbles((b) => [
          ...b,
          {
            kind: "system",
            text: `Switch to chain ${tx.chainId} first. ${
              err instanceof Error ? err.message : ""
            }`,
          },
        ]);
        return;
      }
    }
    try {
      const hash = await sendTransactionAsync({
        to: tx.to,
        data: tx.data,
        value: BigInt(tx.value),
        chainId: tx.chainId,
      });
      setBubbles((b) => [...b, { kind: "system", text: `Broadcast: ${hash}` }]);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setBubbles((b) => [
        ...b,
        { kind: "system", text: `Sign failed: ${msg}` },
      ]);
    }
  }

  async function onClickConnect() {
    if (isConnected) {
      if (
        typeof window !== "undefined" &&
        !window.confirm("Disconnect this wallet?")
      ) {
        return;
      }
      disconnect();
      return;
    }
    const injectedConnector = connectors[0];
    if (!injectedConnector) {
      setBubbles((b) => [
        ...b,
        {
          kind: "system",
          text: "No browser wallet found. Install MetaMask or Rabby and reload.",
        },
      ]);
      return;
    }
    connect({ connector: injectedConnector });
  }

  const hasUserMessage = bubbles.some((b) => b.kind === "user");

  return (
    <>
      <div className="bg-layer bg-glow" />
      <div className="bg-layer bg-grid" />
      <div className="bg-layer bg-noise" />

      <div className="shell">
        <header className="site-header" data-screen-label="header">
          <div className="header-inner wrap">
            <BrandLogo />
            <div className="header-right">
              {isConnected && !onWrongChain && (
                <NetworkBadge chainId={monadMainnet.id} />
              )}
              <AccountButton
                isConnected={isConnected}
                address={address}
                onClick={onClickConnect}
              />
            </div>
          </div>
        </header>

        <main className="scroll" data-screen-label="one-shot">
          <section className="hero">
            <p className="eyebrow">
              <span className="line" />
              Natural language on Monad
            </p>
            <h1 className="hero-title">
              One prompt.
              <br />
              <em>One Monad action.</em>
            </h1>
            <p className="hero-sub">
              Say what you want in plain words — <span className="q">&quot;wrap 0.1 MON&quot;</span> — and Aomi
              builds the transaction. You sign it on Monad testnet. No contract addresses, no gas
              math.
            </p>
            <StarterChips examples={EXAMPLES} onPick={submit} />
          </section>

          {onWrongChain && (
            <WrongChainBanner
              chainId={chainId as number}
              onSwitch={() => switchChainAsync({ chainId: monadMainnet.id })}
            />
          )}
          {connectError && <ErrorBanner message={connectError.message} />}

          <section className="thread">
            {!hasUserMessage && <EmptyState />}
            {bubbles.map((b, i) => (
              <ChatBubble key={i} bubble={b} />
            ))}
            <div ref={endRef} />
          </section>
        </main>

        <div className="composer-zone">
          <div className="composer-fade" />
          <div className="composer-bar">
            <div className="composer-inner">
              {busy && <ThinkingIndicator />}
              <form
                className="composer"
                autoComplete="off"
                onSubmit={(e) => {
                  e.preventDefault();
                  submit(input);
                }}
              >
                <div className="inputwrap">
                  <input
                    value={input}
                    onChange={(e) => setInput(e.target.value)}
                    placeholder='Try: "wrap 0.1 MON"'
                  />
                </div>
                <button className="send" type="submit" disabled={busy || !input.trim()}>
                  Send
                </button>
              </form>
            </div>
          </div>
        </div>
      </div>
    </>
  );
}
