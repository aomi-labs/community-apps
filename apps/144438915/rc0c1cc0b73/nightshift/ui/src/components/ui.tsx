import * as React from "react";

export function cn(...parts: (string | false | null | undefined)[]): string {
  return parts.filter(Boolean).join(" ");
}

export function Panel({
  children,
  className,
  raised,
}: {
  children: React.ReactNode;
  className?: string;
  raised?: boolean;
}) {
  return (
    <section
      className={cn(
        "rounded-2xl border",
        className,
      )}
      style={{
        background: raised
          ? "var(--aomi-surface-raised)"
          : "var(--aomi-surface)",
        borderColor: "var(--aomi-border)",
      }}
    >
      {children}
    </section>
  );
}

export function PanelHeader({
  eyebrow,
  title,
  right,
}: {
  eyebrow?: string;
  title: React.ReactNode;
  right?: React.ReactNode;
}) {
  return (
    <div
      className="flex items-start justify-between gap-4 px-5 py-4 border-b"
      style={{ borderColor: "var(--aomi-hairline)" }}
    >
      <div>
        {eyebrow && (
          <div
            className="font-mono text-[10.5px] uppercase mb-1"
            style={{
              letterSpacing: "var(--aomi-tracking-wide)",
              color: "var(--aomi-text-subtle)",
            }}
          >
            {eyebrow}
          </div>
        )}
        <h2
          className="font-display text-lg leading-tight"
          style={{ color: "var(--aomi-text)" }}
        >
          {title}
        </h2>
      </div>
      {right}
    </div>
  );
}

type BadgeTone = "neutral" | "success" | "danger" | "accent" | "warning";

const badgeTones: Record<BadgeTone, { bg: string; fg: string; bd: string }> = {
  neutral: {
    bg: "color-mix(in srgb, var(--aomi-cool-400) 12%, transparent)",
    fg: "var(--aomi-text-secondary)",
    bd: "color-mix(in srgb, var(--aomi-cool-400) 26%, transparent)",
  },
  success: {
    bg: "color-mix(in srgb, var(--aomi-success) 14%, transparent)",
    fg: "var(--aomi-success)",
    bd: "color-mix(in srgb, var(--aomi-success) 34%, transparent)",
  },
  danger: {
    bg: "color-mix(in srgb, var(--aomi-danger) 15%, transparent)",
    fg: "var(--aomi-danger)",
    bd: "color-mix(in srgb, var(--aomi-danger) 36%, transparent)",
  },
  accent: {
    bg: "var(--aomi-accent-subtle)",
    fg: "var(--aomi-accent)",
    bd: "color-mix(in srgb, var(--aomi-accent) 40%, transparent)",
  },
  warning: {
    bg: "color-mix(in srgb, var(--aomi-warning) 14%, transparent)",
    fg: "var(--aomi-warning)",
    bd: "color-mix(in srgb, var(--aomi-warning) 34%, transparent)",
  },
};

export function Badge({
  children,
  tone = "neutral",
  mono,
}: {
  children: React.ReactNode;
  tone?: BadgeTone;
  mono?: boolean;
}) {
  const t = badgeTones[tone];
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 rounded-pill border px-2.5 py-1 text-xs whitespace-nowrap",
        mono && "font-mono text-[11px]",
      )}
      style={{ background: t.bg, color: t.fg, borderColor: t.bd }}
    >
      {children}
    </span>
  );
}

type BtnVariant = "primary" | "accent" | "outline" | "ghost" | "connect";

export function Button({
  children,
  variant = "primary",
  className,
  ...props
}: {
  children: React.ReactNode;
  variant?: BtnVariant;
} & React.ButtonHTMLAttributes<HTMLButtonElement>) {
  const base =
    "inline-flex items-center justify-center gap-2 whitespace-nowrap font-sans font-medium select-none rounded-pill h-10 px-5 text-sm transition-all duration-200 disabled:opacity-45 disabled:pointer-events-none active:translate-y-px";
  const styles: React.CSSProperties =
    variant === "primary"
      ? { background: "var(--aomi-primary)", color: "var(--aomi-text-on-brand)" }
      : variant === "accent"
        ? { background: "var(--aomi-accent-interactive)", color: "var(--aomi-text-on-brand)" }
        : variant === "outline"
          ? {
              background: "transparent",
              color: "var(--aomi-text)",
              border: "1px solid var(--aomi-border-strong)",
            }
          : variant === "connect"
            ? {
                background: "var(--aomi-bg-subtle)",
                color: "var(--aomi-text-secondary)",
                border: "1px dashed var(--aomi-border-strong)",
              }
            : { background: "transparent", color: "var(--aomi-text)" };
  return (
    <button className={cn(base, className)} style={styles} {...props}>
      {children}
    </button>
  );
}

export function LiveDot({ tone = "success" }: { tone?: "success" | "accent" | "warning" }) {
  const color =
    tone === "success"
      ? "var(--aomi-success)"
      : tone === "warning"
        ? "var(--aomi-warning)"
        : "var(--aomi-accent)";
  return (
    <span
      className="live-dot inline-block h-1.5 w-1.5 rounded-full"
      style={{ background: color, boxShadow: `0 0 0 3px color-mix(in srgb, ${color} 20%, transparent)` }}
    />
  );
}

export function Row({
  label,
  value,
  mono,
  tone,
}: {
  label: string;
  value: React.ReactNode;
  mono?: boolean;
  tone?: "default" | "muted";
}) {
  return (
    <div
      className="flex items-baseline justify-between gap-4 py-2 border-b last:border-b-0"
      style={{ borderColor: "var(--aomi-hairline)" }}
    >
      <span className="text-sm" style={{ color: "var(--aomi-text-muted)" }}>
        {label}
      </span>
      <span
        className={cn("text-sm text-right", mono && "font-mono")}
        style={{
          color: tone === "muted" ? "var(--aomi-text-muted)" : "var(--aomi-text)",
        }}
      >
        {value}
      </span>
    </div>
  );
}
