import { defineChain, type Chain } from "viem";

export const monadMainnet: Chain = defineChain({
  id: 143,
  name: "Monad",
  nativeCurrency: { name: "Monad", symbol: "MON", decimals: 18 },
  rpcUrls: {
    default: { http: ["https://rpc.monad.xyz"] },
  },
  blockExplorers: {
    default: {
      name: "Monadscan",
      url: "https://monadscan.com",
    },
  },
});

export const networks = [monadMainnet] as const;
