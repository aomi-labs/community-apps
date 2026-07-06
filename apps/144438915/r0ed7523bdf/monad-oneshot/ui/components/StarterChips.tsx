/** Intentional starter prompts. onPick fires the message — same handler shape as before. */
export function StarterChips({
  examples,
  onPick,
}: {
  examples: string[];
  onPick: (message: string) => void;
}) {
  return (
    <div className="chips">
      {examples.map((e) => (
        <button key={e} type="button" className="chip" onClick={() => onPick(e)}>
          <span className="glyph">→</span>
          <span className="label">{e}</span>
        </button>
      ))}
    </div>
  );
}
