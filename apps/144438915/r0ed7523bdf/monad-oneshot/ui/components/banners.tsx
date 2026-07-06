/** Calm but unmistakable callout when the wallet is on the wrong network. */
export function WrongChainBanner({
  chainId,
  onSwitch,
}: {
  chainId: number;
  onSwitch: () => void;
}) {
  return (
    <div className="banner warn">
      <span className="ico" />
      <span className="body">
        Your wallet is on another network (chain {chainId}).{" "}
        <b>Switch to Monad testnet</b> to sign.
      </span>
      <button className="switch" type="button" onClick={onSwitch}>
        Switch to Monad
      </button>
    </div>
  );
}

/** Slim error callout for a failed wallet connection. */
export function ErrorBanner({ message }: { message: string }) {
  return (
    <div className="banner danger">
      <span className="ico" />
      <span className="body">Connect failed: {message}</span>
    </div>
  );
}
