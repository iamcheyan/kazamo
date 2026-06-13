export interface Theme {
  bg: string;
  surface: string;
  surfaceAlt: string;
  border: string;
  text: string;
  textSecondary: string;
  muted: string;
  accent: string;
  accentHover: string;
  danger: string;
  dangerBg: string;
  green: string;
  headerBg: string;
  windowCloseHover: string;
  inputBg: string;
}

export const light: Theme = {
  bg: "#fafafa",
  surface: "#ffffff",
  surfaceAlt: "#f0f0f0",
  border: "#e0e0e0",
  text: "#1a1a1a",
  textSecondary: "#666666",
  muted: "#999999",
  accent: "#3584e4",
  accentHover: "#2a72d0",
  danger: "#e53935",
  dangerBg: "#fce4e4",
  green: "#33a852",
  headerBg: "#f6f5f4",
  windowCloseHover: "#e53935",
  inputBg: "#ffffff",
};

export const dark: Theme = {
  bg: "#1e1e1e",
  surface: "#2d2d2d",
  surfaceAlt: "#383838",
  border: "#404040",
  text: "#e0e0e0",
  textSecondary: "#a0a0a0",
  muted: "#707070",
  accent: "#3584e4",
  accentHover: "#4a94f0",
  danger: "#ff5555",
  dangerBg: "#3d1f1f",
  green: "#50c878",
  headerBg: "#252525",
  windowCloseHover: "#c62828",
  inputBg: "#353535",
};

export function getSystemTheme(): "light" | "dark" {
  if (typeof window !== "undefined" && window.matchMedia) {
    return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  }
  return "light";
}

export function resolveTheme(setting: string): Theme {
  if (setting === "dark") return dark;
  if (setting === "light") return light;
  return getSystemTheme() === "dark" ? dark : light;
}
