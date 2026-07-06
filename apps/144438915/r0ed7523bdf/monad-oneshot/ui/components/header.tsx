import { AomiMark } from "./AomiMark";

/** aomi × Monad One-Shot — the × is where the two brand colors meet. */
export function BrandLogo() {
  return (
    <div className="brand">
      <AomiMark className="brand-mark" />
      <span className="brand-word">
        <span className="aomi">aomi</span>
        <span className="cross">×</span>
        <span className="monad">Monad</span>
        <span className="shot">One-Shot</span>
      </span>
    </div>
  );
}

/** Unobtrusive network indicator, shown only when connected and on Monad. */
export function NetworkBadge({ chainId }: { chainId: number }) {
  return (
    <span className="netbadge">
      <span className="pip" />
      Monad testnet · {chainId}
    </span>
  );
}

/** Connect button when disconnected; short address (click to disconnect) when connected. */
export function AccountButton({
  isConnected,
  address,
  onClick,
}: {
  isConnected: boolean;
  address?: `0x${string}`;
  onClick: () => void;
}) {
  if (isConnected && address) {
    return (
      <button
        className="btn account"
        onClick={onClick}
        title="Click to disconnect"
        aria-label={`Connected as ${address}. Click to disconnect.`}
      >
        <span className="dot" />
        <span className="addr">
          {address.slice(0, 6)}…{address.slice(-4)}
        </span>
        <span className="x">×</span>
      </button>
    );
  }
  return (
    <button className="btn" onClick={onClick}>
      Connect wallet
    </button>
  );
}
