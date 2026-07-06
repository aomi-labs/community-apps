// Thin wrapper around the Aomi runtime's `/chat` HTTP endpoint.
//
// The runtime accepts a POST with the user's message, app id, conversation
// history, and a free-form `context` object that gets forwarded to the agent.
// We use `context.user_address` to tell the plugin which wallet is acting.
//
// Response shape (best-effort — adapt to your runtime version):
//   {
//     "message": "Wrapped 0.5 MON. Tx: 0x…",
//     "tool_calls": [{ "name": "wrap_mon", "result": { … } }],
//     "transaction": { "to": "0x…", "data": "0x…", "value": "0x…", "chainId": 143 }
//   }
//
// `transaction` is optional. When present, the page hands it to the connected
// wallet (wagmi injected connector) to sign and broadcast.

export type ChatMessage = { role: "user" | "assistant"; content: string };

export type UnsignedTx = {
  to: `0x${string}`;
  data: `0x${string}`;
  value: `0x${string}`;
  chainId: number;
};

export type ChatResponse = {
  message: string;
  toolCalls: Array<{ name: string; result?: unknown }>;
  transaction?: UnsignedTx;
};

export type SendChatArgs = {
  backendUrl: string;
  appId: string;
  message: string;
  history: ChatMessage[];
  userAddress?: string;
};

export async function sendChat({
  backendUrl,
  appId,
  message,
  history,
  userAddress,
}: SendChatArgs): Promise<ChatResponse> {
  const res = await fetch(`${backendUrl.replace(/\/$/, "")}/chat`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      app_id: appId,
      message,
      history,
      context: userAddress ? { user_address: userAddress } : {},
    }),
  });
  if (!res.ok) {
    const body = await res.text().catch(() => "");
    throw new Error(`chat ${res.status}: ${body || res.statusText}`);
  }
  const data = await res.json();
  return normalize(data);
}

function normalize(raw: unknown): ChatResponse {
  const obj = (raw ?? {}) as Record<string, unknown>;
  const message =
    typeof obj.message === "string"
      ? obj.message
      : typeof obj.reply === "string"
        ? obj.reply
        : "";
  const toolCallsRaw = Array.isArray(obj.tool_calls) ? obj.tool_calls : [];
  const toolCalls = toolCallsRaw.map((tc) => {
    if (typeof tc === "string") return { name: tc };
    const o = tc as Record<string, unknown>;
    return { name: String(o.name ?? ""), result: o.result };
  });
  const tx = (obj.transaction ?? findTxInToolCalls(toolCallsRaw)) as
    | UnsignedTx
    | undefined;
  return { message, toolCalls, transaction: tx };
}

function findTxInToolCalls(calls: unknown[]): UnsignedTx | undefined {
  for (const c of calls) {
    if (typeof c !== "object" || c === null) continue;
    const result = (c as Record<string, unknown>).result as
      | Record<string, unknown>
      | undefined;
    if (
      result &&
      typeof result.to === "string" &&
      typeof result.data === "string" &&
      typeof result.value === "string"
    ) {
      return {
        to: result.to as `0x${string}`,
        data: result.data as `0x${string}`,
        value: result.value as `0x${string}`,
        chainId: Number(result.chainId ?? 143),
      };
    }
  }
  return undefined;
}
