import { createConfig, http } from "wagmi";
import { injected } from "wagmi/connectors";
import { monadMainnet } from "@/lib/chains";

export const wagmiConfig = createConfig({
  chains: [monadMainnet],
  connectors: [injected()],
  transports: {
    [monadMainnet.id]: http(),
  },
  ssr: true,
});
