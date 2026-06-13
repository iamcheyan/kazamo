import { useState, useRef, useEffect } from "react";
import type { Theme } from "./themes";

// ════════════════════════════════════════
//  Design Tokens
// ════════════════════════════════════════
export const C = {
  bg: "#111113",
  sidebar: "#18181b",
  surface: "#1e1e22",
  surfaceHover: "#27272a",
  border: "#2e2e33",
  borderFocus: "#3b82f6",
  text: "#e4e4e7",
  textSecondary: "#a1a1aa",
  muted: "#71717a",
  mutedDark: "#52525b",
  accent: "#3b82f6",
  accentHover: "#2563eb",
  danger: "#ef4444",
  dangerHover: "#dc2626",
  green: "#4ade80",
  greenBg: "#052e16",
  titleBar: "#141416",
  titleBarHover: "#27272a",
  closeHover: "#b91c1c",
  overlay: "#09090b",
};

// ════════════════════════════════════════
//  Select
// ════════════════════════════════════════
interface SelectOption { value: string; label: string; }

export function Select({ value, onChange, options, width, theme }: {
  value: string;
  onChange: (v: string) => void;
  options: SelectOption[];
  width?: number | string;
  theme?: Theme;
}) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const selected = options.find((o) => o.value === value);
  const t = theme || { bg: C.bg, surface: C.surface, surfaceAlt: C.surfaceHover, border: C.border, text: C.text, textSecondary: C.textSecondary, muted: C.muted, accent: C.accent, inputBg: C.surface } as any;

  useEffect(() => {
    const handler = (e: MouseEvent) => { if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false); };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, []);

  return (
    <div ref={ref} style={{ position: "relative", width: width || "100%", maxWidth: 320 }}>
      <button onClick={() => setOpen(!open)}
        style={{
          width: "100%", padding: "7px 12px", borderRadius: 8,
          border: `1px solid ${open ? t.accent : t.border}`,
          background: t.surface, color: t.text, fontSize: 13,
          cursor: "pointer", textAlign: "left",
          display: "flex", alignItems: "center", justifyContent: "space-between",
          transition: "border-color 0.15s",
          outline: "none",
        }}
      >
        <span>{selected?.label || value}</span>
        <span style={{ color: t.muted, fontSize: 10, marginLeft: 8, transform: open ? "rotate(180deg)" : "none", transition: "transform 0.15s" }}>▼</span>
      </button>
      {open && (
        <div style={{
          position: "absolute", top: "calc(100% + 4px)", left: 0, right: 0, zIndex: 50,
          background: t.surface, border: `1px solid ${t.border}`, borderRadius: 8,
          boxShadow: "0 8px 30px rgba(0,0,0,0.5)", overflow: "hidden",
          animation: "fadeSlideIn 0.12s ease-out",
        }}>
          {options.map((opt) => (
            <button key={opt.value} onClick={() => { onChange(opt.value); setOpen(false); }}
              onMouseEnter={(e) => { (e.currentTarget as HTMLElement).style.background = t.surfaceAlt; }}
              onMouseLeave={(e) => { (e.currentTarget as HTMLElement).style.background = opt.value === value ? t.surfaceAlt : "transparent"; }}
              style={{
                width: "100%", padding: "7px 12px", border: "none", cursor: "pointer",
                fontSize: 13, textAlign: "left",
                background: opt.value === value ? t.surfaceAlt : "transparent",
                color: opt.value === value ? t.text : t.textSecondary,
                fontWeight: opt.value === value ? 500 : 400,
                display: "flex", alignItems: "center", justifyContent: "space-between",
              }}
            >
              <span>{opt.label}</span>
              {opt.value === value && <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke={t.accent} strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"><path d="M20 6 9 17l-5-5"/></svg>}
            </button>
          ))}
        </div>
      )}
      <style>{`
        @keyframes fadeSlideIn { from { opacity: 0; transform: translateY(-4px); } to { opacity: 1; transform: translateY(0); } }
      `}</style>
    </div>
  );
}

// ════════════════════════════════════════
//  Button
// ════════════════════════════════════════
export function Button({ children, onClick, variant = "default", disabled, size = "md", theme }: {
  children: React.ReactNode;
  onClick?: () => void;
  variant?: "default" | "primary" | "danger" | "ghost";
  disabled?: boolean;
  size?: "sm" | "md";
  theme?: Theme;
}) {
  const [hovered, setHovered] = useState(false);
  const t = theme || { surface: C.surface, surfaceAlt: C.surfaceHover, border: C.border, textSecondary: C.textSecondary, muted: C.muted, accent: C.accent, accentHover: C.accentHover, danger: C.danger } as any;

  const styles: Record<string, React.CSSProperties> = {
    default: { background: hovered ? t.surfaceAlt : t.surface, color: t.textSecondary, border: `1px solid ${t.border}` },
    primary: { background: hovered ? t.accentHover : t.accent, color: "#fff", border: "none" },
    danger: { background: hovered ? `${t.danger}25` : `${t.danger}15`, color: t.danger, border: `1px solid ${t.danger}40` },
    ghost: { background: "transparent", color: t.muted, border: "none" },
  };

  const paddings = size === "sm" ? "6px 12px" : "8px 18px";
  const fontSizes = size === "sm" ? 12 : 13;

  return (
    <button onClick={onClick} disabled={disabled}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      style={{
        padding: paddings, borderRadius: 8, cursor: disabled ? "not-allowed" : "pointer",
        fontSize: fontSizes, fontWeight: 500, transition: "all 0.12s",
        opacity: disabled ? 0.5 : 1,
        ...styles[variant],
      }}
    >{children}</button>
  );
}

// ════════════════════════════════════════
//  Input
// ════════════════════════════════════════
export function Input({ value, onChange, placeholder, width, mono }: {
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
  width?: number | string;
  mono?: boolean;
}) {
  const [focused, setFocused] = useState(false);
  return (
    <input value={value} onChange={(e) => onChange(e.target.value)} placeholder={placeholder}
      onFocus={() => setFocused(true)} onBlur={() => setFocused(false)}
      style={{
        padding: "10px 14px", borderRadius: 8,
        border: `1px solid ${focused ? C.borderFocus : C.border}`,
        background: C.surface, color: C.text, fontSize: 14,
        outline: "none", width: width || "100%", maxWidth: 320,
        fontFamily: mono ? "monospace" : "inherit",
        transition: "border-color 0.15s",
      }}
    />
  );
}

// ════════════════════════════════════════
//  Badge
// ════════════════════════════════════════
export function Badge({ children, color = C.green }: { children: React.ReactNode; color?: string }) {
  return (
    <span style={{
      fontSize: 11, padding: "2px 8px", borderRadius: 4,
      background: `${color}20`, color, fontWeight: 500,
      display: "inline-flex", alignItems: "center", gap: 4,
    }}>{children}</span>
  );
}

// ════════════════════════════════════════
//  Tabs
// ════════════════════════════════════════
export function Tabs({ value, onChange, options }: {
  value: string;
  onChange: (v: string) => void;
  options: { value: string; label: string }[];
}) {
  return (
    <div style={{ display: "flex", gap: 2, background: C.surface, borderRadius: 8, padding: 3, border: `1px solid ${C.border}` }}>
      {options.map((opt) => {
        const active = opt.value === value;
        return (
          <button key={opt.value} onClick={() => onChange(opt.value)}
            style={{
              padding: "7px 16px", borderRadius: 6, border: "none", cursor: "pointer",
              fontSize: 13, fontWeight: 500, transition: "all 0.15s",
              background: active ? C.accent : "transparent",
              color: active ? "#fff" : C.muted,
            }}
          >{opt.label}</button>
        );
      })}
    </div>
  );
}
