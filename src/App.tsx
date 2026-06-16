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
type Provider = "sensevoice" | "paraformer";
type RecordingState = "idle" | "recording" | "processing";
interface ModelInfo { name: string; downloaded: boolean; path: string; size_mb: number; }

const appWindow = getCurrentWindow();

const LANGUAGES = [
  { value: "auto", label: <>{Icon.globe(13)} Auto</> },
  { value: "zh", label: <>中 Chinese</> },
  { value: "en", label: <>EN English</> },
  { value: "ja", label: <>日 Japanese</> },
  { value: "ko", label: <>한 Korean</> },
  { value: "yue", label: <>粤 Cantonese</> },
];

const THEME_OPTIONS = [
  { value: "system", label: <>{Icon.monitor(13)} System</> },
  { value: "light", label: <>{Icon.sun(13)} Light</> },
  { value: "dark", label: <>{Icon.moon(13)} Dark</> },
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

const MODEL_DETAILS: Record<string, { desc: string; tag?: string; tagColor?: string }> = {
  "SenseVoice Small Q3_K": { desc: "3-bit 量化，极速，体积最小，准确率稍低", tag: "极速", tagColor: "#a1a1aa" },
  "SenseVoice Small Q4_0": { desc: "4-bit 标准量化，速度快，内存占用低", tag: "标准", tagColor: "#3b82f6" },
  "SenseVoice Small Q4_1": { desc: "4-bit 优化量化，内存占用低，准确度比 Q4_0 稍好", tag: "标准", tagColor: "#3b82f6" },
  "SenseVoice Small Q4_K": { desc: "4-bit 混合量化 (K-means)，速度与准确率高度平衡", tag: "推荐", tagColor: "#10b981" },
  "SenseVoice Small Q5_0": { desc: "5-bit 标准量化，准确率好，速度较快", tag: "高精度", tagColor: "#8b5cf6" },
  "SenseVoice Small Q5_K": { desc: "5-bit 混合量化 (K-means)，高准确率，运行流畅", tag: "推荐", tagColor: "#10b981" },
  "SenseVoice Small Q6_K": { desc: "6-bit 混合量化，极度接近无损 FP16 准确度", tag: "超清", tagColor: "#f59e0b" },
  "SenseVoice Small Q8_0": { desc: "8-bit 高精度量化，准确度极高，适合高配置电脑", tag: "极清", tagColor: "#ef4444" },
  "SenseVoice Small FP16": { desc: "16-bit 半精度浮点，无损音质，运算速度较慢", tag: "无损", tagColor: "#ec4899" },
  "SenseVoice Small FP32": { desc: "32-bit 全精度浮点，完整未压缩模型，最慢", tag: "完整", tagColor: "#6b7280" },
  "SenseVoice Small ONNX INT8": { desc: "sherpa-onnx INT8 模型，适合 Linux aarch64 原生运行", tag: "ARM64", tagColor: "#10b981" },
  "Paraformer-Large": { desc: "阿里开源高精度中文识别模型，适合中文长语音", tag: "中文特化", tagColor: "#f59e0b" },
};

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
        <div className="headerbar-left">
          {page !== "main" ? (
            <Mie.Button
              icon={<span className="icon-wrap">{Icon.arrowLeft(14)}</span>}
              transparent
              onClick={() => setPage("main")}
              size="tiny"
            />
          ) : (
            <>
              <Mie.Button
                icon={<span className="icon-wrap">{Icon.package(14)}</span>}
                transparent
                active={false}
                onClick={() => setPage("models")}
                size="tiny"
              />
              <Mie.Button
                icon={<span className="icon-wrap">{Icon.settings(14)}</span>}
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
    <div className="settings-section-label">{text}</div>
  );

  return (
    <div className="settings-page">
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
          icon={<span className="settings-link-icon">{Icon.helpCircle(15)}</span>}
          side={<span className="settings-link-arrow">›</span>}
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
  const [modelsInitialTab, setModelsInitialTab] = useState<Provider>("sensevoice");
  const [themeMode, setThemeMode] = useState("system");
  const [theme, setTheme] = useState<Theme>(getSystemTheme() === "dark" ? dark : light);
  const [zoom, setZoom] = useState(() => {
    try { return Number(localStorage.getItem("kazamo-zoom")) || 100; } catch { return 100; }
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

  // Apply theme to body + CSS variables
  useEffect(() => {
    document.body.style.background = "transparent";
    document.body.style.color = theme.text;
    const actualTheme = themeMode === "system" ? (getSystemTheme() === "dark" ? "dark" : "light") : themeMode;
    document.body.setAttribute("data-theme", actualTheme);
    document.documentElement.setAttribute("data-theme", actualTheme);
    // Set CSS custom properties for theme colors
    const root = document.documentElement;
    root.style.setProperty("--bg", theme.bg);
    root.style.setProperty("--surface", theme.surface);
    root.style.setProperty("--surface-alt", theme.surfaceAlt);
    root.style.setProperty("--border", theme.border);
    root.style.setProperty("--text", theme.text);
    root.style.setProperty("--text-secondary", theme.textSecondary);
    root.style.setProperty("--muted", theme.muted);
    root.style.setProperty("--accent", theme.accent);
    root.style.setProperty("--accent-hover", theme.accentHover);
    root.style.setProperty("--danger", theme.danger);
    root.style.setProperty("--danger-bg", theme.dangerBg);
    root.style.setProperty("--green", theme.green);
    root.style.setProperty("--header-bg", theme.headerBg);
    root.style.setProperty("--input-bg", theme.inputBg);
    root.style.setProperty("--green-bg", `${theme.green}12`);
    root.style.setProperty("--danger-border", `${theme.danger}33`);
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

  // Set window size strictly to 525x582 and apply browser zoom (adjusted by zoom factor)
  useEffect(() => {
    const scale = zoom / 100;
    const baseW = 525;
    const baseH = 582;
    const newW = baseW * scale;
    const newH = baseH * scale;

    // Apply Chromium's native webview zoom (perfect vector scaling of all pixel dimensions)
    try {
      getCurrentWebview().setZoom(scale).catch(() => {});
    } catch {}

    const resizeWindow = async () => {
      try {
        // 200ms delay to prevent GTK SIGTRAP / crash during early window mapping on Linux
        await new Promise((resolve) => setTimeout(resolve, 200));
        await appWindow.setResizable(true);
        await appWindow.setSize(new LogicalSize(newW, newH));
        await appWindow.center();
        await appWindow.setResizable(false);
      } catch (e) {
        console.error("Failed to resize window:", e);
      }
    };
    resizeWindow();
  }, [zoom]);

  const setThemeModeAndApply = (mode: string) => {
    setThemeMode(mode);
    const t = mode === "dark" ? dark : mode === "light" ? light : getSystemTheme() === "dark" ? dark : light;
    setTheme(t);
  };

  const navigate = (nextPage: Page, modelTab?: Provider) => {
    if (modelTab) setModelsInitialTab(modelTab);
    setPage(nextPage);
  };

  return (
    <ThemeCtx.Provider value={{ theme, mode: themeMode === "system" ? getSystemTheme() : themeMode, setThemeMode: setThemeModeAndApply }}>
      <Mie.Window className="kazamo-window">
        <HeaderBar page={page} setPage={setPage} />
        <main className="page-viewport">
          {page === "main" && <MainPage onNavigate={navigate} />}
          {page === "models" && <ModelsPage initialTab={modelsInitialTab} />}
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
function MainPage({ onNavigate }: { onNavigate: (p: Page, modelTab?: Provider) => void }) {
  const [state, setState] = useState<RecordingState>("idle");
  const [transcript, setTranscript] = useState("");
  const [error, setError] = useState("");
  const [copied, setCopied] = useState(false);
  const [provider, setProvider] = useState<Provider>("sensevoice");
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [settings, setSettings] = useState<any>(null);
  const stateRef = useRef(state);
  stateRef.current = state;

  // Load settings + models
  useEffect(() => {
    invoke("get_settings").then((s: any) => {
      setProvider((s.provider || "sensevoice") as Provider);
      setSettings(s);
    }).catch(() => {});
    invoke("list_models").then((list: any) => setModels(list)).catch(() => {});
  }, []);

  // Listen for model selection changes from Manage Models page so main page display updates immediately
  useEffect(() => {
    const handler = () => {
      invoke("get_settings").then((s: any) => {
        setProvider((s.provider || "sensevoice") as Provider);
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

  const currentModel = activeModel || downloadedForProvider[0];
  const hasModel = !!currentModel;
  const missingModelMessage = provider === "sensevoice"
    ? "Download a SenseVoice model before recording."
    : "Download a Paraformer model before recording.";

  const saveProvider = async (p: Provider) => {
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

  const doStart = useCallback(async (fresh?: { provider: Provider; settings: any; models: ModelInfo[] }) => {
    const startProvider = fresh?.provider ?? provider;
    const startSettings = fresh?.settings ?? settings;
    const startModels = fresh?.models ?? models;
    const startActiveModelName = startProvider === "sensevoice"
      ? startSettings?.sensevoice_model
      : startSettings?.paraformer_model;
    const startActiveModel = startModels.find(m => m.name === startActiveModelName && m.downloaded);
    const startDownloadedForProvider = startModels.filter(m =>
      m.downloaded && (
        startProvider === "sensevoice" ? m.name.includes("SenseVoice") : m.name.includes("Paraformer")
      )
    );
    const startCurrentModel = startActiveModel || startDownloadedForProvider[0];
    const startMissingModelMessage = startProvider === "sensevoice"
      ? "Download a SenseVoice model before recording."
      : "Download a Paraformer model before recording.";

    if (fresh) {
      setProvider(startProvider);
      setSettings(startSettings);
      setModels(startModels);
    }

    setError(""); setTranscript(""); setCopied(false);
    if (!startCurrentModel) {
      setError(startMissingModelMessage);
      return false;
    }
    try {
      if (startCurrentModel && startCurrentModel.name !== startActiveModelName && startSettings) {
        const nextSettings = {
          ...startSettings,
          sensevoice_model: startProvider === "sensevoice" ? startCurrentModel.name : startSettings.sensevoice_model,
          paraformer_model: startProvider === "paraformer" ? startCurrentModel.name : startSettings.paraformer_model,
        };
        await invoke("save_settings", {
          language: startSettings.language,
          provider: startSettings.provider,
          hotkey: startSettings.hotkey,
          theme: startSettings.theme,
          sensevoice_model: nextSettings.sensevoice_model,
          paraformer_model: nextSettings.paraformer_model,
        });
        setSettings(nextSettings);
      }
      await invoke("start_recording");
      setState("recording");
    }
    catch (e: any) { setError(`${e}`); return false; }
    return true;
  }, [models, provider, settings]);

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
    const u1 = listen("ipc-toggle-start", async () => {
      if (stateRef.current !== "idle") return;
      let started = false;
      try {
        const s: any = await invoke("get_settings");
        const list: ModelInfo[] = await invoke("list_models");
        started = await doStart({
          provider: (s.provider || "sensevoice") as Provider,
          settings: s,
          models: list,
        });
      } catch {
        started = await doStart();
      }
      if (!started) {
        await invoke("set_ipc_result", { text: "error" }).catch(() => {});
        await invoke("stop_recording").catch(() => {});
      } else {
        await invoke("set_ipc_result", { text: "started" }).catch(() => {});
      }
    });
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
    <div className="main-page">
      {/* Record button */}
      <div className="record-control">
        <button onClick={toggle} disabled={state === "processing" || !hasModel}
          className={`record-button ${state}`}
          aria-disabled={!hasModel || state === "processing"}
          title={!hasModel ? missingModelMessage : undefined}
          style={{
            background: state === "recording" ? "var(--danger)" : state === "processing" || !hasModel ? "var(--muted)" : "var(--accent)",
            boxShadow: state === "recording" ? "0 0 0 4px var(--danger-glow)" : !hasModel ? "none" : "0 0 0 4px var(--accent-glow)"
          }}
        >{state === "recording" ? Icon.stop(18) : state === "processing" ? Icon.spinner(18) : Icon.mic(18)}</button>
        <p className="record-hint">
          {state === "recording" ? "Recording..." : state === "processing" ? "Transcribing..." : hasModel ? "Press to record" : "Download a model to record"}
        </p>
      </div>

      {error && <div className="error-banner">{error}</div>}

      {/* Provider tabs */}
      <div className="provider-tabs">
        {(["sensevoice", "paraformer"] as Provider[]).map((p) => (
          <button key={p} onClick={() => saveProvider(p)}
            className={`provider-tab-btn${provider === p ? " active" : ""}`}
          >{p === "sensevoice" ? "SenseVoice" : "Paraformer"}</button>
        ))}
      </div>

      {/* Provider description */}
      <div className="provider-desc">
        {provider === "sensevoice"
          ? "SenseVoice: 多语言极速识别 (中/英/日/韩/粤)，支持检测情绪及笑声、掌声、BGM等事件。"
          : "Paraformer: 专注中文高准确率语音识别，适合大段中文长语音识别。"}
      </div>

      {/* Transcription (Editable text box) */}
      <textarea
        className={`transcription-panel${transcript ? " has-text" : ""}`}
        value={transcript}
        onChange={(e) => setTranscript(e.target.value)}
        placeholder="Transcription will appear here..."
      />

      <div className="transcript-footer" style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 10 }}>
        {/* Active model info / download prompt */}
        <div style={{ minWidth: 0, flex: 1 }}>
          {hasModel ? (
            <div className="model-status" style={{ fontSize: 11, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
              <span className="icon-wrap" style={{ color: "var(--muted)", flexShrink: 0 }}>
                {Icon.package(12)}
              </span>
              <span className="model-status-label" style={{ flexShrink: 0 }}>Using:</span>
              <span className="model-status-name" style={{ flexShrink: 0 }}>
                {currentModel?.name || "—"}
              </span>
            </div>
          ) : (
            <div className="no-model" style={{ fontSize: 11 }}>
              <span className="status-dot danger" style={{ flexShrink: 0 }} />
              <span style={{ flexShrink: 0 }}>No model downloaded.</span>
              <button onClick={() => onNavigate("models", provider)}
                className="download-link" style={{ fontSize: 11, flexShrink: 0 }}
              >Download now</button>
            </div>
          )}
        </div>

        {/* Copy button */}
        <div className="transcript-footer-inner" style={{ flexShrink: 0 }}>
          {transcript && (
            <>
              {copied && <span className="copied-label">{Icon.check(11)} Copied</span>}
              <button onClick={() => { navigator.clipboard.writeText(transcript); setCopied(true); setTimeout(() => setCopied(false), 1500); }}
                className="copy-btn"
              >{Icon.clipboard(11)} Copy</button>
            </>
          )}
        </div>
      </div>

      {/* Status bar */}
      <div className="status-bar">
        <div className="status-bar-left">
          <span className={`status-indicator${state === "recording" || !hasModel ? " recording" : " ready"}`} />
          {state === "recording" ? "Recording" : state === "processing" ? "Processing" : hasModel ? "Ready" : "Model required"}
        </div>
        <button onClick={() => onNavigate("help")}
          className="hotkey-link"
        >Setup hotkey</button>
      </div>
    </div>
  );
}

// ════════════════════════════════════════
//  Models Page
// ════════════════════════════════════════
function ModelsPage({ initialTab }: { initialTab: Provider }) {
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [loading, setLoading] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [progress, setProgress] = useState<Record<string, { percent?: number; file?: string; status: string }>>({});
  const [activeTab, setActiveTab] = useState<Provider>(initialTab);
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
    setActiveTab(initialTab);
  }, [initialTab]);

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
    <div className="models-page">
      {/* Tabs */}
      <div className="provider-tabs" style={{ marginBottom: 12 }}>
        {(["sensevoice", "paraformer"] as Provider[]).map((p) => (
          <button key={p} onClick={() => setActiveTab(p)}
            className={`provider-tab-btn${activeTab === p ? " active" : ""}`}
          >{p === "sensevoice" ? "SenseVoice" : "Paraformer"}</button>
        ))}
      </div>

      {error && <div className="models-error">{error}</div>}

      {/* Flat list */}
      <div className="model-list">
        {filteredModels.map((m) => {
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
              className={`model-item${isActive ? " active" : ""}${effectiveDownloaded ? " clickable" : ""}`}
            >
              {/* Name + size + description */}
              <div className="model-info">
                <div className="model-name-row">
                  <span className={`model-name${isActive ? " active-name" : ""}`}>{m.name}</span>
                  {MODEL_DETAILS[m.name]?.tag && (
                    <span className="model-tag" style={{
                      background: `${MODEL_DETAILS[m.name].tagColor}22`,
                      color: MODEL_DETAILS[m.name].tagColor,
                      border: `1px solid ${MODEL_DETAILS[m.name].tagColor}33`,
                    }}>{MODEL_DETAILS[m.name].tag}</span>
                  )}
                  {effectiveDownloaded && (
                    <span className="model-size">{m.size_mb}MB</span>
                  )}
                </div>
                {MODEL_DETAILS[m.name] && (
                  <span className="model-desc">{MODEL_DETAILS[m.name].desc}</span>
                )}
              </div>

              {/* Progress bar (inline) */}
              {showProgress && (
                <div className="progress-wrap">
                  <div className="progress-track">
                    <div className="progress-fill" style={{ width: `${pct}%` }} />
                  </div>
                  <div className="progress-label">{pct}%</div>
                </div>
              )}

              {/* Actions */}
              <div className="model-actions" onClick={(e) => e.stopPropagation()}>
                {effectiveDownloaded ? (
                  <>
                    {isActive && (
                      <span className="model-active-check">{Icon.check(10)}</span>
                    )}
                    <button
                      onClick={(e) => { e.stopPropagation(); del(m.name); }}
                      disabled={isBusy}
                      className="model-action-btn delete"
                    >{isBusy ? Icon.spinner(10) : Icon.trash(10)}</button>
                  </>
                ) : showProgress ? (
                  <span className="model-spinner">{prog?.status === "complete" ? Icon.check(12) : Icon.spinner(12)}</span>
                ) : (
                  <button
                    onClick={(e) => { e.stopPropagation(); download(m.name); }}
                    disabled={loading !== null}
                    className="model-action-btn download"
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
  return (
    <div className="help-page">
      <h3>Global Hotkey Setup</h3>
      <p>
        Add this to <code>~/.config/labwc/rc.xml</code>:
      </p>
      <div className="help-code-block">
        <pre>{`<keybind key="A-r">
  <action name="Execute">
    <command>kazamo toggle</command>
  </action>
</keybind>`}</pre>
      </div>
      <p>
        Press <strong>Alt+R</strong> to toggle recording. Auto-starts Kazamo if not running.
      </p>
      <p>
        Or run <code>kazamo toggle</code> from the terminal.
      </p>
    </div>
  );
}
