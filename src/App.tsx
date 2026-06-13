import { useState, useEffect, useRef, useCallback, createContext, useContext } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Icon } from "./icons";
import { light, dark, getSystemTheme, type Theme } from "./themes";

type Page = "main" | "models" | "help";
type RecordingState = "idle" | "recording" | "processing";
interface ModelInfo { name: string; downloaded: boolean; path: string; size_mb: number; }

const appWindow = getCurrentWindow();
const radius = 12;

const LANGUAGES = [
  { value: "auto", label: "Auto", flag: "🌐" },
  { value: "zh", label: "Chinese", flag: "🇨🇳" },
  { value: "en", label: "English", flag: "🇺🇸" },
  { value: "ja", label: "Japanese", flag: "🇯🇵" },
  { value: "ko", label: "Korean", flag: "🇰🇷" },
  { value: "yue", label: "Cantonese", flag: "🇭🇰" },
];

const ThemeCtx = createContext<{ theme: Theme; mode: string; toggle: () => void }>({ theme: light, mode: "light", toggle: () => {} });
const useT = () => useContext(ThemeCtx);

// ════════════════════════════════════════
//  HeaderBar
// ════════════════════════════════════════
function HeaderBar({ page, setPage }: { page: Page; setPage: (p: Page) => void }) {
  const { theme: T, mode, toggle } = useT();
  const [showMenu, setShowMenu] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const h = (e: MouseEvent) => { if (menuRef.current && !menuRef.current.contains(e.target as Node)) setShowMenu(false); };
    document.addEventListener("mousedown", h); return () => document.removeEventListener("mousedown", h);
  }, []);

  const winBtn = (icon: React.ReactNode, hoverBg: string, onClick: () => void) => (
    <button onClick={onClick}
      style={{ width: 32, height: 32, border: "none", background: "transparent", color: T.textSecondary, cursor: "pointer", borderRadius: 8, display: "flex", alignItems: "center", justifyContent: "center" }}
      onMouseEnter={(e) => { (e.currentTarget as HTMLElement).style.background = hoverBg; }}
      onMouseLeave={(e) => { (e.currentTarget as HTMLElement).style.background = "transparent"; }}
    >{icon}</button>
  );

  // Sun/Moon icon
  const themeIcon = mode === "dark"
    ? <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round"><circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M6.34 17.66l-1.41 1.41M19.07 4.93l-1.41 1.41"/></svg>
    : <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/></svg>;

  return (
    <div data-tauri-drag-region style={{ height: 46, background: T.headerBg, display: "flex", alignItems: "center", borderBottom: `1px solid ${T.border}`, flexShrink: 0, paddingLeft: 8, paddingRight: 8, gap: 4 }}>
      {page !== "main" && (
        <button onClick={() => setPage("main")}
          style={{ width: 32, height: 32, border: "none", background: "transparent", color: T.textSecondary, cursor: "pointer", borderRadius: 8, display: "flex", alignItems: "center", justifyContent: "center" }}
          onMouseEnter={(e) => { (e.currentTarget as HTMLElement).style.background = T.surfaceAlt; }}
          onMouseLeave={(e) => { (e.currentTarget as HTMLElement).style.background = "transparent"; }}
        >{Icon.arrowLeft(16)}</button>
      )}
      <div data-tauri-drag-region style={{ flex: 1, fontSize: 13, fontWeight: 600, color: T.text, paddingLeft: 4 }}>
        {page === "main" ? "Kazamo" : page === "models" ? "Models" : "Help"}
      </div>

      {/* Theme toggle */}
      {winBtn(themeIcon, T.surfaceAlt, toggle)}

      {page === "main" && (
        <div ref={menuRef} style={{ position: "relative" }}>
          <button onClick={() => setShowMenu(!showMenu)}
            style={{ width: 32, height: 32, border: "none", background: showMenu ? T.surfaceAlt : "transparent", color: T.textSecondary, cursor: "pointer", borderRadius: 8, display: "flex", alignItems: "center", justifyContent: "center" }}
            onMouseEnter={(e) => { (e.currentTarget as HTMLElement).style.background = T.surfaceAlt; }}
            onMouseLeave={(e) => { (e.currentTarget as HTMLElement).style.background = showMenu ? T.surfaceAlt : "transparent"; }}
          >{Icon.settings(16)}</button>
          {showMenu && (
            <div style={{ position: "absolute", top: 38, right: 0, zIndex: 50, minWidth: 150, background: T.surface, border: `1px solid ${T.border}`, borderRadius: radius, boxShadow: mode === "dark" ? "0 4px 20px rgba(0,0,0,0.4)" : "0 4px 20px rgba(0,0,0,0.12)", overflow: "hidden" }}>
              {[["models", Icon.package(15), "Models"], ["help", <svg key="h" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round"><circle cx="12" cy="12" r="10"/><path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/><line x1="12" x2="12.01" y1="17" y2="17"/></svg>, "Help"]].map(([key, icon, label]) => (
                <button key={key as string} onClick={() => { setPage(key as Page); setShowMenu(false); }}
                  style={{ width: "100%", padding: "10px 14px", border: "none", cursor: "pointer", fontSize: 13, textAlign: "left", background: "transparent", color: T.text, display: "flex", alignItems: "center", gap: 10 }}
                  onMouseEnter={(e) => { (e.currentTarget as HTMLElement).style.background = T.surfaceAlt; }}
                  onMouseLeave={(e) => { (e.currentTarget as HTMLElement).style.background = "transparent"; }}
                >{icon}{label}</button>
              ))}
            </div>
          )}
        </div>
      )}
      {winBtn(<svg width="14" height="14" viewBox="0 0 24 24" stroke="currentColor" strokeWidth="2" fill="none"><line x1="5" x2="19" y1="12" y2="12"/></svg>, T.surfaceAlt, () => appWindow.minimize())}
      {winBtn(<svg width="12" height="12" viewBox="0 0 24 24" stroke="currentColor" strokeWidth="2.5" fill="none"><path d="M18 6 6 18M6 6l12 12"/></svg>, T.windowCloseHover, () => appWindow.close())}
    </div>
  );
}

// ════════════════════════════════════════
//  App
// ════════════════════════════════════════
export default function App() {
  const [page, setPage] = useState<Page>("main");
  const [themeMode, setThemeMode] = useState("system");
  const [theme, setTheme] = useState<Theme>(getSystemTheme() === "dark" ? dark : light);

  // Load saved theme
  useEffect(() => {
    invoke("get_settings").then((s: any) => {
      const mode = s.theme || "system";
      setThemeMode(mode);
      setTheme(mode === "dark" ? dark : mode === "light" ? light : getSystemTheme() === "dark" ? dark : light);
    }).catch(() => {});
  }, []);

  // Listen for system theme changes
  useEffect(() => {
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = () => { if (themeMode === "system") setTheme(mq.matches ? dark : light); };
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  }, [themeMode]);

  const toggleTheme = async () => {
    const next = themeMode === "light" ? "dark" : themeMode === "dark" ? "system" : "light";
    setThemeMode(next);
    const t = next === "dark" ? dark : next === "light" ? light : getSystemTheme() === "dark" ? dark : light;
    setTheme(t);
    try {
      const s: any = await invoke("get_settings");
      await invoke("save_settings", { language: s.language, provider: s.provider, hotkey: s.hotkey, theme: next });
    } catch {}
  };

  // Apply theme to body
  useEffect(() => {
    document.body.style.background = theme.bg;
    document.body.style.color = theme.text;
  }, [theme]);

  return (
    <ThemeCtx.Provider value={{ theme, mode: themeMode === "system" ? getSystemTheme() : themeMode, toggle: toggleTheme }}>
      <div style={{ display: "flex", flexDirection: "column", height: "100vh", fontFamily: "Cantarell, 'Noto Sans', sans-serif", background: theme.bg, color: theme.text, fontSize: 14, transition: "background 0.2s, color 0.2s" }}>
        <HeaderBar page={page} setPage={setPage} />
        <div style={{ flex: 1, overflow: "auto" }}>
          {page === "main" && <MainPage onNavigate={setPage} />}
          {page === "models" && <ModelsPage />}
          {page === "help" && <HelpPage />}
        </div>
      </div>
    </ThemeCtx.Provider>
  );
}

// ════════════════════════════════════════
//  Main Page
// ════════════════════════════════════════
function MainPage({ onNavigate }: { onNavigate: (p: Page) => void }) {
  const { theme: T } = useT();
  const [state, setState] = useState<RecordingState>("idle");
  const [transcript, setTranscript] = useState("");
  const [error, setError] = useState("");
  const [copied, setCopied] = useState(false);
  const [provider, setProvider] = useState("sensevoice");
  const [language, setLanguage] = useState("auto");
  const stateRef = useRef(state);
  stateRef.current = state;

  useEffect(() => { invoke("get_settings").then((s: any) => { setProvider(s.provider || "sensevoice"); setLanguage(s.language || "auto"); }).catch(() => {}); }, []);

  const save = async (p: string, l: string) => { try { const s: any = await invoke("get_settings"); await invoke("save_settings", { language: l, provider: p, hotkey: s.hotkey, theme: s.theme }); } catch {} };

  const doStop = useCallback(async () => {
    setState("processing"); setError("");
    try {
      const bytes: number[] = await invoke("stop_recording");
      if (bytes.length < 100) { setError("Recording too short"); setState("idle"); return; }
      const r: any = await invoke("transcribe_audio", { audioData: bytes });
      if (r.success) { setTranscript(r.text); navigator.clipboard.writeText(r.text).catch(() => {}); setCopied(true); setTimeout(() => setCopied(false), 2000); }
      else setError(r.error || "Transcription failed");
    } catch (e: any) { setError(`${e}`); }
    setState("idle");
  }, []);

  const doStart = useCallback(async () => {
    setError(""); setTranscript(""); setCopied(false);
    try { await invoke("start_recording"); setState("recording"); }
    catch (e: any) { setError(`${e}`); }
  }, []);

  const toggle = useCallback(() => {
    if (stateRef.current === "recording") doStop();
    else if (stateRef.current === "idle") doStart();
  }, [doStart, doStop]);

  useEffect(() => {
    const h = (e: KeyboardEvent) => {
      if ((e.target as HTMLElement).tagName === "INPUT" || (e.target as HTMLElement).tagName === "SELECT") return;
      if (e.code === "Space" && !e.ctrlKey && !e.altKey && !e.metaKey) { e.preventDefault(); toggle(); }
    };
    window.addEventListener("keydown", h); return () => window.removeEventListener("keydown", h);
  }, [toggle]);

  useEffect(() => {
    const u1 = listen("ipc-toggle-start", () => { if (stateRef.current === "idle") doStart(); });
    const u2 = listen("ipc-toggle-stop", async () => {
      if (stateRef.current !== "recording") return;
      setState("processing"); setError("");
      try {
        const bytes: number[] = await invoke("stop_recording");
        if (bytes.length < 100) { await invoke("set_ipc_result", { text: "error" }); setState("idle"); return; }
        const r: any = await invoke("transcribe_audio", { audioData: bytes });
        if (r.success) { setTranscript(r.text); navigator.clipboard.writeText(r.text).catch(() => {}); setCopied(true); setTimeout(() => setCopied(false), 2000); await invoke("set_ipc_result", { text: r.text }); }
        else { setError(r.error || "Failed"); await invoke("set_ipc_result", { text: "error" }); }
      } catch (e: any) { setError(`${e}`); await invoke("set_ipc_result", { text: "error" }); }
      setState("idle");
    });
    return () => { u1.then((f) => f()); u2.then((f) => f()); };
  }, [doStart]);

  return (
    <div style={{ padding: "16px 20px", display: "flex", flexDirection: "column", height: "100%" }}>
      {/* Record button */}
      <div style={{ display: "flex", flexDirection: "column", alignItems: "center", padding: "12px 0 10px" }}>
        <button onClick={toggle} disabled={state === "processing"}
          style={{ width: 60, height: 60, borderRadius: "50%", border: "none", cursor: state === "processing" ? "wait" : "pointer", background: state === "recording" ? T.danger : state === "processing" ? T.muted : T.accent, color: "#fff", transition: "all 0.2s", display: "flex", alignItems: "center", justifyContent: "center", boxShadow: state === "recording" ? `0 0 0 4px ${T.danger}33` : `0 0 0 4px ${T.accent}22` }}
        >{state === "recording" ? Icon.stop(20) : state === "processing" ? Icon.spinner(20) : Icon.mic(20)}</button>
        <p style={{ marginTop: 6, fontSize: 12, color: T.muted }}>
          {state === "recording" ? "Recording..." : state === "processing" ? "Transcribing..." : "Press to record"}
        </p>
      </div>

      {error && <div style={{ padding: "8px 12px", borderRadius: 8, background: T.dangerBg, color: T.danger, fontSize: 12, marginBottom: 8 }}>{error}</div>}

      {/* Provider tabs */}
      <div style={{ display: "flex", gap: 2, background: T.surfaceAlt, borderRadius: 8, padding: 2, marginBottom: 8, transition: "background 0.2s" }}>
        {["sensevoice", "paraformer"].map((p) => (
          <button key={p} onClick={() => { setProvider(p); save(p, language); }}
            style={{ flex: 1, padding: "6px 0", borderRadius: 6, border: "none", cursor: "pointer", fontSize: 12, fontWeight: 500, background: provider === p ? T.surface : "transparent", color: provider === p ? T.text : T.muted, boxShadow: provider === p ? "0 1px 3px rgba(0,0,0,0.1)" : "none", transition: "all 0.15s" }}
          >{p === "sensevoice" ? "SenseVoice" : "Paraformer"}</button>
        ))}
      </div>

      {/* Transcription */}
      <div style={{ flex: 1, display: "flex", flexDirection: "column" }}>
        <div style={{ flex: 1, borderRadius: radius, border: `1px solid ${T.border}`, background: T.surface, padding: 12, overflow: "auto", minHeight: 60, transition: "background 0.2s, border-color 0.2s" }}>
          <p style={{ fontSize: 13, color: transcript ? T.text : T.muted, fontStyle: transcript ? "normal" : "italic", lineHeight: 1.7, whiteSpace: "pre-wrap" }}>{transcript || "Transcription will appear here..."}</p>
        </div>

        <div style={{ marginTop: 6, display: "flex", alignItems: "center", justifyContent: "space-between" }}>
          <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
            {transcript && (
              <>
                {copied && <span style={{ fontSize: 11, color: T.green, display: "flex", alignItems: "center", gap: 3 }}>{Icon.check(11)} Copied</span>}
                <button onClick={() => { navigator.clipboard.writeText(transcript); setCopied(true); setTimeout(() => setCopied(false), 1500); }}
                  style={{ padding: "4px 10px", borderRadius: 6, border: `1px solid ${T.border}`, background: T.surface, color: T.textSecondary, fontSize: 11, cursor: "pointer", display: "flex", alignItems: "center", gap: 4, transition: "all 0.15s" }}
                  onMouseEnter={(e) => { (e.currentTarget as HTMLElement).style.background = T.surfaceAlt; }}
                  onMouseLeave={(e) => { (e.currentTarget as HTMLElement).style.background = T.surface; }}
                >{Icon.clipboard(11)} Copy</button>
              </>
            )}
          </div>
          <select value={language}
            onChange={(e) => { setLanguage(e.target.value); save(provider, e.target.value); }}
            style={{ padding: "4px 8px", borderRadius: 6, border: `1px solid ${T.border}`, background: T.inputBg, color: T.textSecondary, fontSize: 11, cursor: "pointer", outline: "none", transition: "all 0.2s" }}
          >
            {LANGUAGES.map((l) => <option key={l.value} value={l.value}>{l.flag} {l.label}</option>)}
          </select>
        </div>
      </div>

      {/* Status bar */}
      <div style={{ marginTop: 8, paddingTop: 6, borderTop: `1px solid ${T.border}`, display: "flex", alignItems: "center", justifyContent: "space-between", fontSize: 11, color: T.muted, transition: "border-color 0.2s" }}>
        <div style={{ display: "flex", alignItems: "center", gap: 5 }}>
          <span style={{ width: 6, height: 6, borderRadius: "50%", background: state === "recording" ? T.danger : T.green, display: "inline-block", transition: "background 0.2s" }} />
          {state === "recording" ? "Recording" : state === "processing" ? "Processing" : "Ready"}
        </div>
        <button onClick={() => onNavigate("help")}
          style={{ background: "none", border: "none", color: T.accent, fontSize: 11, cursor: "pointer", padding: 0, textDecoration: "underline" }}
        >Setup hotkey</button>
      </div>
    </div>
  );
}

// ════════════════════════════════════════
//  Models Page
// ════════════════════════════════════════
function ModelsPage() {
  const { theme: T } = useT();
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [loading, setLoading] = useState<string | null>(null);
  useEffect(() => { invoke("list_models").then(setModels as any).catch(() => {}); }, []);
  const del = async (name: string) => { setLoading(name); try { await invoke("delete_model", { name }); invoke("list_models").then(setModels as any).catch(() => {}); } catch {} setLoading(null); };

  return (
    <div style={{ padding: "16px 20px" }}>
      {models.map((m) => (
        <div key={m.name} style={{ border: `1px solid ${T.border}`, borderRadius: radius, padding: 14, background: T.surface, marginBottom: 10, transition: "all 0.2s" }}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <div>
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <span style={{ fontWeight: 600, fontSize: 14, color: T.text }}>{m.name}</span>
                {m.downloaded && <span style={{ fontSize: 11, padding: "2px 6px", borderRadius: 4, background: `${T.green}18`, color: T.green }}>{Icon.check(10)} {m.size_mb}MB</span>}
              </div>
              <p style={{ fontSize: 12, color: T.muted, marginTop: 2 }}>{m.name === "SenseVoice" ? "Multi-language speech recognition" : "Chinese speech recognition"}</p>
            </div>
            {m.downloaded ? (
              <button onClick={() => del(m.name)} disabled={loading === m.name}
                style={{ padding: "6px 12px", borderRadius: 8, border: `1px solid ${T.border}`, background: T.surface, color: T.danger, fontSize: 12, cursor: "pointer", display: "flex", alignItems: "center", gap: 4 }}
              >{loading === m.name ? Icon.spinner(12) : Icon.trash(12)}</button>
            ) : (
              <button style={{ padding: "6px 14px", borderRadius: 8, border: "none", background: T.accent, color: "#fff", fontSize: 12, cursor: "pointer", display: "flex", alignItems: "center", gap: 4 }}
              >{Icon.download(12)} Download</button>
            )}
          </div>
        </div>
      ))}
    </div>
  );
}

// ════════════════════════════════════════
//  Help Page
// ════════════════════════════════════════
function HelpPage() {
  const { theme: T } = useT();
  return (
    <div style={{ padding: "20px" }}>
      <h3 style={{ fontSize: 15, fontWeight: 600, marginBottom: 12 }}>Global Hotkey Setup</h3>
      <p style={{ fontSize: 13, color: T.textSecondary, lineHeight: 1.7, marginBottom: 14 }}>
        Add this to <code style={{ padding: "1px 4px", borderRadius: 3, background: T.surfaceAlt, fontSize: 12 }}>~/.config/labwc/rc.xml</code>:
      </p>
      <div style={{ borderRadius: radius, border: `1px solid ${T.border}`, background: T.surfaceAlt, padding: 14, marginBottom: 14, transition: "all 0.2s" }}>
        <pre style={{ fontSize: 12, fontFamily: "monospace", color: T.text, lineHeight: 1.7, margin: 0, whiteSpace: "pre-wrap" }}>{`<keybind key="A-r">
  <action name="Execute">
    <command>kazamo toggle</command>
  </action>
</keybind>`}</pre>
      </div>
      <p style={{ fontSize: 13, color: T.textSecondary, lineHeight: 1.7, marginBottom: 10 }}>
        Press <strong>Alt+R</strong> to toggle recording. Auto-starts Kazamo if not running.
      </p>
      <p style={{ fontSize: 12, color: T.muted, lineHeight: 1.6 }}>
        Or run <code style={{ padding: "1px 4px", borderRadius: 3, background: T.surfaceAlt, fontSize: 11, fontFamily: "monospace" }}>kazamo toggle</code> from the terminal.
      </p>
    </div>
  );
}
