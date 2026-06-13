import { useState, useEffect, useRef, useCallback, createContext, useContext } from "react";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Icon } from "./icons";
import { light, dark, getSystemTheme, type Theme } from "./themes";
import { Select } from "./ui";
import * as Mie from "@mielo-ui/mielo-react";

type Page = "main" | "models" | "help" | "settings";
type RecordingState = "idle" | "recording" | "processing";
interface ModelInfo { name: string; downloaded: boolean; path: string; size_mb: number; }

const appWindow = getCurrentWindow();
const radius = 12;
const BASE_WINDOW = { width: 520, height: 560 };
const SUPPORTED_ZOOMS = [100, 110, 125, 150] as const;

function normalizeZoom(value: number) {
  return SUPPORTED_ZOOMS.includes(value as (typeof SUPPORTED_ZOOMS)[number]) ? value : 100;
}

const LANGUAGES = [
  { value: "auto", label: "🌐 Auto" },
  { value: "zh", label: "🇨🇳 Chinese" },
  { value: "en", label: "🇺🇸 English" },
  { value: "ja", label: "🇯🇵 Japanese" },
  { value: "ko", label: "🇰🇷 Korean" },
  { value: "yue", label: "🇭🇰 Cantonese" },
];

const THEME_OPTIONS = [
  { value: "system", label: "🖥️ System" },
  { value: "light", label: "☀️ Light" },
  { value: "dark", label: "🌙 Dark" },
];

const ZOOM_OPTIONS = [
  { value: "100", label: "100%" },
  { value: "125", label: "125%" },
  { value: "150", label: "150%" },
  { value: "175", label: "175%" },
  { value: "200", label: "200%" },
  { value: "225", label: "225%" },
  { value: "250", label: "250%" },
  { value: "275", label: "275%" },
  { value: "300", label: "300%" },
];

const ThemeCtx = createContext<{ theme: Theme; mode: string; setThemeMode: (m: string) => void }>({ theme: light, mode: "light", setThemeMode: () => {} });
const useT = () => useContext(ThemeCtx);

// ════════════════════════════════════════
//  HeaderBar
// ════════════════════════════════════════
function HeaderBar({ page, setPage }: { page: Page; setPage: (p: Page) => void }) {
  const handleControlClick = (_event: any, control: "minimize" | "maximize" | "restore" | "close") => {
    if (control === "close") {
      appWindow.close().catch(() => {});
    } else if (control === "minimize") {
      appWindow.minimize().catch(() => {});
    }
  };

  const titleText = page === "main" ? "Kazamo" : page === "models" ? "Models" : page === "settings" ? "Settings" : "Help";

  return (
    <Mie.HeaderBar
      className="kazamo-headerbar"
      data-tauri-drag-region
      header={<Mie.Header title={titleText} />}
      left={
        <div style={{ display: "flex", gap: 4 }}>
          {page !== "main" ? (
            <Mie.Button
              icon={<span style={{ display: "flex", alignItems: "center" }}>{Icon.arrowLeft(14)}</span>}
              transparent
              onClick={() => setPage("main")}
              size="tiny"
            />
          ) : (
            <>
              <Mie.Button
                icon={<span style={{ display: "flex", alignItems: "center" }}>{Icon.package(14)}</span>}
                transparent
                active={false}
                onClick={() => setPage("models")}
                size="tiny"
              />
              <Mie.Button
                icon={<span style={{ display: "flex", alignItems: "center" }}>{Icon.settings(14)}</span>}
                transparent
                active={false}
                onClick={() => setPage("settings")}
                size="tiny"
              />
            </>
          )}
        </div>
      }
      controls={
        <Mie.WindowControls
          controls={["minimize", "close"]}
          onClickControl={handleControlClick}
        />
      }
    />
  );
}

// ════════════════════════════════════════
//  Settings Page
// ════════════════════════════════════════
function SettingsPage({ zoom, setZoom }: { zoom: number; setZoom: (z: number) => void }) {
  const { theme: T, setThemeMode } = useT();
  const [themeMode, setLocalThemeMode] = useState("system");
  const [language, setLanguage] = useState("auto");
  useEffect(() => {
    invoke("get_settings").then((s: any) => {
      setLocalThemeMode(s.theme || "system");
      setLanguage(s.language || "auto");
    }).catch(() => {});
  }, []);

  const handleThemeChange = async (mode: string) => {
    setLocalThemeMode(mode);
    setThemeMode(mode);
    try {
      const s: any = await invoke("get_settings");
      await invoke("save_settings", { language: s.language, provider: s.provider, hotkey: s.hotkey, theme: mode });
    } catch {}
  };

  const handleLanguageChange = async (lang: string) => {
    setLanguage(lang);
    try {
      const s: any = await invoke("get_settings");
      await invoke("save_settings", { language: lang, provider: s.provider, hotkey: s.hotkey, theme: s.theme });
    } catch {}
  };

  const handleZoomChange = (z: string) => {
    const num = Number(z);
    setZoom(num);
    try { localStorage.setItem("kazamo-zoom", String(num)); } catch {}
  };

  const sectionLabel = (text: string) => (
    <div className="settings-section-label" style={{ color: T.muted }}>{text}</div>
  );

  return (
    <div className="page settings-page">
      {/* ── Theme & Zoom ── */}
      {sectionLabel("Appearance")}
      <Mie.Rows className="settings-rows">
        <Mie.Rows.Row
          title="Theme"
          description="Choose light, dark, or follow system"
          side={<Select value={themeMode} onChange={handleThemeChange} options={THEME_OPTIONS} width="140px" theme={T} />}
        />
        <Mie.Rows.Row
          title="Zoom"
          description="Adjust interface scaling"
          side={<Select value={String(zoom)} onChange={handleZoomChange} options={ZOOM_OPTIONS} width="140px" theme={T} />}
        />
      </Mie.Rows>

      {/* ── Language ── */}
      {sectionLabel("Recognition")}
      <Mie.Rows className="settings-rows">
        <Mie.Rows.Row
          title="Language"
          description="Speech recognition language"
          side={<Select value={language} onChange={handleLanguageChange} options={LANGUAGES} width="140px" theme={T} />}
        />
      </Mie.Rows>

      {/* ── Links ── */}
      {sectionLabel("More")}
      <Mie.Rows className="settings-rows">
        <Mie.Rows.Row
          title="Hotkey Setup"
          description="Configure global hotkey"
          activatable
          hover
          onClick={() => {
            const ev = new CustomEvent("kazamo-navigate", { detail: "help" });
            window.dispatchEvent(ev);
          }}
          icon={<span style={{ display: "flex", alignItems: "center", color: T.textSecondary }}>{Icon.helpCircle(15)}</span>}
          side={<span style={{ color: T.muted, fontSize: 14 }}>›</span>}
        />
      </Mie.Rows>
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
  const [zoom, setZoom] = useState(() => {
    try { return normalizeZoom(Number(localStorage.getItem("kazamo-zoom"))); } catch { return 100; }
  });

  // Load saved settings
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

  // Apply theme to body
  useEffect(() => {
    document.body.style.background = theme.bg;
    document.body.style.color = theme.text;
    const actualTheme = themeMode === "system" ? (getSystemTheme() === "dark" ? "dark" : "light") : themeMode;
    document.body.setAttribute("data-theme", actualTheme);
    document.documentElement.setAttribute("data-theme", actualTheme);
  }, [theme, themeMode]);

  // Listen for navigation events from SettingsPage links
  useEffect(() => {
    const handler = (e: Event) => {
      const detail = (e as CustomEvent).detail as Page;
      if (detail) setPage(detail);
    };
    window.addEventListener("kazamo-navigate", handler);
    return () => window.removeEventListener("kazamo-navigate", handler);
  }, []);

  // Update window size on zoom change and center the window
  useEffect(() => {
    const scale = zoom / 100;
    const newW = Math.round(BASE_WINDOW.width * scale);
    const newH = Math.round(BASE_WINDOW.height * scale);

    // Set webview zoom factor
    try {
      getCurrentWebview().setZoom(scale).catch(() => {});
    } catch {}

    appWindow.setSize(new LogicalSize(newW, newH)).then(() => {
      appWindow.center().catch(() => {});
    }).catch(() => {});
  }, [zoom]);

  const setThemeModeAndApply = (mode: string) => {
    setThemeMode(mode);
    const t = mode === "dark" ? dark : mode === "light" ? light : getSystemTheme() === "dark" ? dark : light;
    setTheme(t);
  };

  return (
    <ThemeCtx.Provider value={{ theme, mode: themeMode === "system" ? getSystemTheme() : themeMode, setThemeMode: setThemeModeAndApply }}>
      <Mie.Window className="kazamo-window">
        <HeaderBar page={page} setPage={setPage} />
        <main className="page-viewport">
          {page === "main" && <MainPage onNavigate={setPage} />}
          {page === "models" && <ModelsPage />}
          {page === "settings" && <SettingsPage zoom={zoom} setZoom={setZoom} />}
          {page === "help" && <HelpPage />}
        </main>
      </Mie.Window>
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
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [settings, setSettings] = useState<any>(null);
  const stateRef = useRef(state);
  stateRef.current = state;

  // Load settings + models
  useEffect(() => {
    invoke("get_settings").then((s: any) => {
      setProvider(s.provider || "sensevoice");
      setSettings(s);
    }).catch(() => {});
    invoke("list_models").then((list: any) => setModels(list)).catch(() => {});
  }, []);

  // Listen for model selection changes from Manage Models page so main page display updates immediately
  useEffect(() => {
    const handler = () => {
      invoke("get_settings").then((s: any) => {
        setProvider(s.provider || "sensevoice");
        setSettings(s);
      }).catch(() => {});
      invoke("list_models").then((list: any) => setModels(list)).catch(() => {});
    };
    window.addEventListener("kazamo-model-changed", handler);
    return () => window.removeEventListener("kazamo-model-changed", handler);
  }, []);

  // Derive active model for current provider
  const activeModelName = provider === "sensevoice"
    ? settings?.sensevoice_model
    : settings?.paraformer_model;

  const activeModel = models.find(m => m.name === activeModelName && m.downloaded);

  // Any downloaded model for current provider?
  const downloadedForProvider = models.filter(m =>
    m.downloaded && (
      provider === "sensevoice" ? m.name.includes("SenseVoice") : m.name.includes("Paraformer")
    )
  );

  const hasModel = !!activeModel || downloadedForProvider.length > 0;

  const saveProvider = async (p: string) => {
    setProvider(p);
    try {
      const s: any = await invoke("get_settings");
      await invoke("save_settings", { language: s.language, provider: p, hotkey: s.hotkey, theme: s.theme });
    } catch {}
  };

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
    <div className="page main-page">
      {/* Record button */}
      <div className="record-control">
        <button onClick={toggle} disabled={state === "processing"}
          className="record-button"
          style={{ cursor: state === "processing" ? "wait" : "pointer", background: state === "recording" ? T.danger : state === "processing" ? T.muted : T.accent, boxShadow: state === "recording" ? `0 0 0 4px ${T.danger}33` : `0 0 0 4px ${T.accent}22` }}
        >{state === "recording" ? Icon.stop(18) : state === "processing" ? Icon.spinner(18) : Icon.mic(18)}</button>
        <p className="record-hint" style={{ color: T.muted }}>
          {state === "recording" ? "Recording..." : state === "processing" ? "Transcribing..." : "Press to record"}
        </p>
      </div>

      {error && <div style={{ padding: "8px 12px", borderRadius: 8, background: T.dangerBg, color: T.danger, fontSize: 12, marginBottom: 8 }}>{error}</div>}

      {/* Provider tabs */}
      <div className="provider-tabs" style={{ background: T.surfaceAlt }}>
        {["sensevoice", "paraformer"].map((p) => (
          <button key={p} onClick={() => saveProvider(p)}
            style={{ flex: 1, padding: "8px 0", borderRadius: 7, border: "none", cursor: "pointer", fontSize: 13, fontWeight: 500, background: provider === p ? T.surface : "transparent", color: provider === p ? T.text : T.muted, boxShadow: provider === p ? "0 1px 3px rgba(0,0,0,0.1)" : "none", transition: "all 0.15s" }}
          >{p === "sensevoice" ? "SenseVoice" : "Paraformer"}</button>
        ))}
      </div>

      {/* Active model info / download prompt */}
      <div className="model-summary">
        {hasModel ? (
          <div style={{ display: "flex", alignItems: "center", gap: 7, fontSize: 13, color: T.textSecondary }}>
            <span style={{ width: 5, height: 5, borderRadius: "50%", background: T.green, display: "inline-block" }} />
            <span style={{ color: T.muted }}>Using:</span>
            <span style={{ fontWeight: 500, color: T.text }}>
              {activeModel
                ? activeModel.name.replace("SenseVoice Small ", "")
                : downloadedForProvider[0]?.name.replace("SenseVoice Small ", "") || "—"}
            </span>
          </div>
        ) : (
          <div style={{ display: "flex", alignItems: "center", gap: 7, fontSize: 13, color: T.danger }}>
            <span style={{ width: 5, height: 5, borderRadius: "50%", background: T.danger, display: "inline-block" }} />
            <span>No model downloaded.</span>
            <button onClick={() => onNavigate("models")}
              style={{ background: "none", border: "none", color: T.accent, fontSize: 13, cursor: "pointer", padding: 0, textDecoration: "underline", fontWeight: 500 }}
            >Download now</button>
          </div>
        )}
      </div>

      {/* Transcription */}
      <div className="transcription-panel" style={{ borderRadius: radius, border: `1px solid ${T.border}`, background: T.surface }}>
        <p style={{ fontSize: 13, color: transcript ? T.text : T.muted, fontStyle: transcript ? "normal" : "italic", lineHeight: 1.65, whiteSpace: "pre-wrap" }}>{transcript || "Transcription will appear here..."}</p>
      </div>

      <div className="transcript-footer">
        <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
          {transcript && (
            <>
              {copied && <span style={{ fontSize: 11, color: T.green, display: "flex", alignItems: "center", gap: 3 }}>{Icon.check(11)} Copied</span>}
              <button onClick={() => { navigator.clipboard.writeText(transcript); setCopied(true); setTimeout(() => setCopied(false), 1500); }}
                style={{ padding: "3px 9px", borderRadius: 6, border: `1px solid ${T.border}`, background: T.surface, color: T.textSecondary, fontSize: 11, cursor: "pointer", display: "flex", alignItems: "center", gap: 4, transition: "all 0.15s" }}
                onMouseEnter={(e) => { (e.currentTarget as HTMLElement).style.background = T.surfaceAlt; }}
                onMouseLeave={(e) => { (e.currentTarget as HTMLElement).style.background = T.surface; }}
              >{Icon.clipboard(11)} Copy</button>
            </>
          )}
        </div>
      </div>

      {/* Status bar */}
      <div className="status-bar" style={{ borderTop: `1px solid ${T.border}`, color: T.muted }}>
        <div style={{ display: "flex", alignItems: "center", gap: 5 }}>
          <span style={{ width: 6, height: 6, borderRadius: "50%", background: state === "recording" ? T.danger : T.green, display: "inline-block", transition: "background 0.2s" }} />
          {state === "recording" ? "Recording" : state === "processing" ? "Processing" : "Ready"}
        </div>
        <button onClick={() => onNavigate("help")}
          style={{ background: "none", border: "none", color: T.accent, fontSize: 12, cursor: "pointer", padding: 0, textDecoration: "underline" }}
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
  const [error, setError] = useState<string | null>(null);
  const [progress, setProgress] = useState<Record<string, { percent?: number; file?: string; status: string }>>({});
  const [activeTab, setActiveTab] = useState<"sensevoice" | "paraformer">("sensevoice");
  const [settings, setSettings] = useState<any>(null);

  const refresh = async () => {
    try {
      const list = await invoke("list_models");
      setModels(list as any);
    } catch {}
  };

  const loadSettings = () => invoke("get_settings").then(setSettings as any).catch(() => {});

  useEffect(() => {
    refresh();
    loadSettings();
  }, []);

  useEffect(() => {
    if (!loading) return;
    const id = setInterval(() => {
      refresh();
    }, 8000);
    return () => clearInterval(id);
  }, [loading]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen("download-progress", (event: any) => {
      const payload = event.payload as { model: string; status: string; percent?: number; file?: string };
      setProgress(prev => ({
        ...prev,
        [payload.model]: { status: payload.status, percent: payload.percent, file: payload.file }
      }));
      if (payload.status === "complete") {
        invoke("list_models").then(setModels as any).catch(() => {});
        setTimeout(() => {
          setProgress(p => {
            const { [payload.model]: _, ...rest } = p;
            return rest;
          });
        }, 650);
      }
    }).then(un => { unlisten = un; });
    return () => { if (unlisten) unlisten(); };
  }, []);

  const del = async (name: string) => {
    setLoading(name);
    setError(null);
    try {
      await invoke("delete_model", { name });
      await refresh();
    } catch (e: any) {
      setError(`Delete failed: ${e}`);
    }
    setLoading(null);
    setProgress(prev => { const { [name]: _, ...rest } = prev; return rest; });
  };

  const download = async (name: string) => {
    setLoading(name);
    setError(null);
    setProgress(prev => {
      const { [name]: _, ...rest } = prev;
      return {
        ...rest,
        [name]: { status: "downloading", percent: 0 }
      };
    });
    try {
      await invoke("download_model", { name });
      await refresh();
      setProgress(prev => { const { [name]: _, ...rest } = prev; return rest; });
    } catch (e: any) {
      setError(`Download failed: ${e}`);
    }
    setLoading(null);
  };

  const selectModel = async (name: string) => {
    if (!settings) return;

    // Optimistic update so highlight moves immediately on click
    const isSenseVoice = name.includes("SenseVoice");
    const updatedSettings = {
      ...settings,
      sensevoice_model: isSenseVoice ? name : settings.sensevoice_model,
      paraformer_model: !isSenseVoice ? name : settings.paraformer_model,
    };
    setSettings(updatedSettings);

    try {
      await invoke("save_settings", {
        language: settings.language,
        provider: settings.provider,
        hotkey: settings.hotkey,
        theme: settings.theme,
        sensevoice_model: updatedSettings.sensevoice_model,
        paraformer_model: updatedSettings.paraformer_model,
      });
      // Notify other pages (e.g. main) to refresh their view of the active model immediately
      window.dispatchEvent(new CustomEvent("kazamo-model-changed"));
      // No need to reload here; optimistic already applied.
      // If you want to re-sync from disk/backend later, can call loadSettings() but it may cause flicker.
    } catch (e) {
      console.error(e);
      // On failure, revert by reloading from backend
      loadSettings();
    }
  };

  const filteredModels = models.filter(m => {
    if (activeTab === "sensevoice") return m.name.includes("SenseVoice");
    if (activeTab === "paraformer") return m.name.includes("Paraformer");
    return true;
  });

  return (
    <div style={{ padding: "12px 16px" }}>
      {/* Tabs */}
      <div style={{ display: "flex", gap: 2, background: T.surfaceAlt, borderRadius: 6, padding: 2, marginBottom: 12 }}>
        {["sensevoice", "paraformer"].map((p) => (
          <button key={p} onClick={() => setActiveTab(p as any)}
            style={{
              flex: 1, padding: "5px 0", borderRadius: 4, border: "none", cursor: "pointer",
              background: activeTab === p ? T.surface : "transparent",
              color: activeTab === p ? T.text : T.muted,
              fontSize: 12, fontWeight: activeTab === p ? 600 : 400,
              boxShadow: activeTab === p ? "0 1px 2px rgba(0,0,0,0.08)" : "none",
              transition: "all 0.15s"
            }}
          >{p === "sensevoice" ? "SenseVoice" : "Paraformer"}</button>
        ))}
      </div>

      {error && (
        <div style={{ padding: "8px 10px", borderRadius: 6, background: `${T.danger}18`, color: T.danger, fontSize: 11, marginBottom: 10, border: `1px solid ${T.danger}33` }}>
          {error}
        </div>
      )}

      {/* Flat list */}
      <div style={{ border: `1px solid ${T.border}`, borderRadius: 8, overflow: "hidden" }}>
        {filteredModels.map((m, i) => {
          const prog = progress[m.name];
          const isBusy = loading === m.name;
          const pct = prog?.percent ?? (prog?.status === "complete" ? 100 : 0);
          const isDownloading = !!prog && (prog.status === "downloading" || prog.status === "complete");
          const effectiveDownloaded = m.downloaded && !isBusy;
          const showProgress = (isBusy || isDownloading) && !effectiveDownloaded;
          const isActive = (activeTab === "sensevoice" && settings?.sensevoice_model === m.name) ||
                           (activeTab === "paraformer" && settings?.paraformer_model === m.name);

          return (
            <div key={m.name}
              onClick={() => { if (effectiveDownloaded) selectModel(m.name); }}
              style={{
                display: "flex", alignItems: "center", gap: 10,
                padding: "8px 12px",
                borderLeft: isActive ? `3px solid ${T.green}` : "3px solid transparent",
                borderBottom: i < filteredModels.length - 1 ? `1px solid ${T.border}` : "none",
                background: isActive ? `${T.green}12` : "transparent",
                cursor: effectiveDownloaded ? "pointer" : "default",
                transition: "background 0.15s, border-color 0.15s",
              }}
              onMouseEnter={(e) => { if (!isActive) (e.currentTarget as HTMLElement).style.background = T.surfaceAlt; }}
              onMouseLeave={(e) => { if (!isActive) (e.currentTarget as HTMLElement).style.background = "transparent"; }}
            >
              {/* Name + size */}
              <div style={{ flex: 1, minWidth: 0, display: "flex", alignItems: "center", gap: 8 }}>
                <span style={{
                  fontSize: 12, fontWeight: isActive ? 600 : 400,
                  color: isActive ? T.green : T.text,
                  overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap"
                }}>{m.name}</span>
                {effectiveDownloaded && (
                  <span style={{ fontSize: 10, color: T.muted, flexShrink: 0 }}>{m.size_mb}MB</span>
                )}
              </div>

              {/* Progress bar (inline) */}
              {showProgress && (
                <div style={{ width: 60, flexShrink: 0 }}>
                  <div style={{ height: 3, background: T.border, borderRadius: 2, overflow: "hidden" }}>
                    <div style={{ height: "100%", width: `${pct}%`, background: T.accent, transition: "width 120ms linear" }} />
                  </div>
                  <div style={{ fontSize: 9, color: T.muted, marginTop: 1, textAlign: "right" }}>{pct}%</div>
                </div>
              )}

              {/* Actions */}
              <div style={{ flexShrink: 0, display: "flex", alignItems: "center", gap: 4 }} onClick={(e) => e.stopPropagation()}>
                {effectiveDownloaded ? (
                  <>
                    {isActive && (
                      <span style={{ fontSize: 10, color: T.green, display: "flex", alignItems: "center" }}>{Icon.check(10)}</span>
                    )}
                    <button
                      onClick={(e) => { e.stopPropagation(); del(m.name); }}
                      disabled={isBusy}
                      style={{ width: 24, height: 24, borderRadius: 4, border: "none", background: "transparent", color: T.muted, cursor: isBusy ? "not-allowed" : "pointer", display: "flex", alignItems: "center", justifyContent: "center", opacity: isBusy ? 0.4 : 1 }}
                      onMouseEnter={(e) => { (e.currentTarget as HTMLElement).style.color = T.danger; }}
                      onMouseLeave={(e) => { (e.currentTarget as HTMLElement).style.color = T.muted; }}
                    >{isBusy ? Icon.spinner(10) : Icon.trash(10)}</button>
                  </>
                ) : showProgress ? (
                  <span style={{ display: "flex", alignItems: "center" }}>{prog?.status === "complete" ? Icon.check(12) : Icon.spinner(12)}</span>
                ) : (
                  <button
                    onClick={(e) => { e.stopPropagation(); download(m.name); }}
                    disabled={loading !== null}
                    style={{ width: 24, height: 24, borderRadius: 4, border: "none", background: "transparent", color: T.accent, cursor: loading !== null ? "not-allowed" : "pointer", display: "flex", alignItems: "center", justifyContent: "center", opacity: loading !== null ? 0.4 : 1 }}
                  >{Icon.download(12)}</button>
                )}
              </div>
            </div>
          );
        })}
      </div>
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
