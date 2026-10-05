/* Imported from VibeCody (MIT, (c) 2026 Ravindra Boddipalli): packages/vibe-ui-shared/src/theme/themes.ts
   Adapted: storage keys/events are namespaced for odoo-rs, and the applied CSS is cached for a flash-free first paint. */
/** Canonical theme registry and CSS applier for all VibeCody webview shells. */

/* ── Types ──────────────────────────────────────────────────────────── */

export interface ThemeDef {
  id: string;
  name: string;
  category: "standard" | "high-contrast" | "color-blind";
  mode: "dark" | "light";
  pairId: string; // links dark/light counterparts
  preview: { bg: string; fg: string; accent: string; secondary: string };
  vars: Record<string, string>;
}

export const THEME_CATEGORIES = [
  { id: "standard", label: "Standard" },
  { id: "high-contrast", label: "High contrast" },
  { id: "color-blind", label: "Color-blind friendly" },
] as const;

/* ── Theme definitions ─────────────────────────────────────────────── */

// Each theme has a pairId linking its dark/light counterpart
/* ── The default ──────────────────────────────────────────────────────────
   Charcoal is what a fresh install opens in, on every shell. Named here
   rather than spelled as a literal at each call site: the fallback lived in
   four places and had already drifted — the toggle booted `dark-sherwood`
   while the editor's fallback said `dark-default`, so a user with no stored
   choice got one palette for the app and another for the code pane.
   `vibecoder/design-system/tokens.css` carries the same palette as its
   `:root`, so the first paint (before any of this runs) is already Charcoal
   rather than a flash of something else. */
export const DEFAULT_DARK_THEME_ID = "dark-charcoal";
export const DEFAULT_LIGHT_THEME_ID = "light-charcoal";

export const THEMES: ThemeDef[] = [
  // ── Pair: Default (Midnight Blue / Clean White) ──
  {
    id: "dark-default", name: "Default", category: "standard", mode: "dark", pairId: "default",
    preview: { bg: "#0f1117", fg: "#e2e4ea", accent: "#6c8cff", secondary: "#161821" },
    vars: {
      "--bg-primary": "#0f1117", "--bg-secondary": "#161821", "--bg-tertiary": "#1c1f2b", "--bg-elevated": "#222638",
      "--text-primary": "#e2e4ea", "--text-secondary": "#6e7491", "--accent-blue": "#6c8cff", "--accent-green": "#34d399",
      "--accent-purple": "#a78bfa", "--accent-gold": "#f5c542", "--accent-rose": "#f472b6",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#ef4444",
    },
  },
  {
    id: "light-default", name: "Default", category: "standard", mode: "light", pairId: "default",
    preview: { bg: "#fafbfd", fg: "#1a1d2e", accent: "#4f6df5", secondary: "#f0f1f5" },
    vars: {
      "--bg-primary": "#fafbfd", "--bg-secondary": "#f0f1f5", "--bg-tertiary": "#e6e8ef", "--bg-elevated": "#ffffff",
      "--text-primary": "#1a1d2e", "--text-secondary": "#6b7089", "--accent-blue": "#4f6df5", "--accent-green": "#10b981",
      "--accent-purple": "#8b5cf6", "--accent-gold": "#d4a017", "--accent-rose": "#ec4899",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#dc2626",
    },
  },
  // ── Pair: Charcoal / Silver ──
  {
    id: "dark-charcoal", name: "Charcoal", category: "standard", mode: "dark", pairId: "charcoal",
    preview: { bg: "#1a1a1a", fg: "#d4d4d4", accent: "#569cd6", secondary: "#252526" },
    vars: {
      "--bg-primary": "#1a1a1a", "--bg-secondary": "#252526", "--bg-tertiary": "#2d2d30", "--bg-elevated": "#333337",
      "--text-primary": "#d4d4d4", "--text-secondary": "#808080", "--accent-blue": "#569cd6", "--accent-green": "#6a9955",
      "--accent-purple": "#c586c0", "--accent-gold": "#dcdcaa", "--accent-rose": "#d7ba7d",
      "--border-color": "rgba(255, 255, 255, 0.05)", "--error-color": "#f14c4c",
    },
  },
  {
    id: "light-charcoal", name: "Charcoal", category: "standard", mode: "light", pairId: "charcoal",
    preview: { bg: "#f3f3f3", fg: "#1e1e1e", accent: "#005fb8", secondary: "#e8e8e8" },
    vars: {
      "--bg-primary": "#f3f3f3", "--bg-secondary": "#e8e8e8", "--bg-tertiary": "#d6d6d6", "--bg-elevated": "#ffffff",
      "--text-primary": "#1e1e1e", "--text-secondary": "#616161", "--accent-blue": "#005fb8", "--accent-green": "#388a34",
      "--accent-purple": "#8839a1", "--accent-gold": "#bf8803", "--accent-rose": "#c72e49",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#cd3131",
    },
  },
  // ── Pair: Warm (Warm Dusk / Warm Sand) ──
  {
    id: "dark-warm", name: "Warm", category: "standard", mode: "dark", pairId: "warm",
    preview: { bg: "#1a1410", fg: "#e6ddd0", accent: "#d4a373", secondary: "#2a2118" },
    vars: {
      "--bg-primary": "#1a1410", "--bg-secondary": "#2a2118", "--bg-tertiary": "#3a2e22", "--bg-elevated": "#453828",
      "--text-primary": "#e6ddd0", "--text-secondary": "#a89880", "--accent-blue": "#d4a373", "--accent-green": "#859900",
      "--accent-purple": "#b58db6", "--accent-gold": "#d4a373", "--accent-rose": "#d33682",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#dc322f",
    },
  },
  {
    id: "light-warm", name: "Warm", category: "standard", mode: "light", pairId: "warm",
    preview: { bg: "#fdf6e3", fg: "#073642", accent: "#268bd2", secondary: "#eee8d5" },
    vars: {
      "--bg-primary": "#fdf6e3", "--bg-secondary": "#eee8d5", "--bg-tertiary": "#e0dbc7", "--bg-elevated": "#fffdf5",
      "--text-primary": "#073642", "--text-secondary": "#586e75", "--accent-blue": "#268bd2", "--accent-green": "#859900",
      "--accent-purple": "#6c71c4", "--accent-gold": "#b58900", "--accent-rose": "#d33682",
      "--border-color": "rgba(0, 0, 0, 0.07)", "--error-color": "#dc322f",
    },
  },
  // ── Pair: Ocean (Deep Ocean / Coastal Light) ──
  {
    id: "dark-ocean", name: "Ocean", category: "standard", mode: "dark", pairId: "ocean",
    preview: { bg: "#0d1b2a", fg: "#e0e1dd", accent: "#48cae4", secondary: "#1b2838" },
    vars: {
      "--bg-primary": "#0d1b2a", "--bg-secondary": "#1b2838", "--bg-tertiary": "#233345", "--bg-elevated": "#2b3e50",
      "--text-primary": "#e0e1dd", "--text-secondary": "#778da9", "--accent-blue": "#48cae4", "--accent-green": "#52b788",
      "--accent-purple": "#b392f0", "--accent-gold": "#ffb703", "--accent-rose": "#ff6b6b",
      "--border-color": "rgba(255, 255, 255, 0.05)", "--error-color": "#ff6b6b",
    },
  },
  {
    id: "light-ocean", name: "Ocean", category: "standard", mode: "light", pairId: "ocean",
    preview: { bg: "#f0f8ff", fg: "#0d1b2a", accent: "#0077b6", secondary: "#e0f0fa" },
    vars: {
      "--bg-primary": "#f0f8ff", "--bg-secondary": "#e0f0fa", "--bg-tertiary": "#c8e3f5", "--bg-elevated": "#ffffff",
      "--text-primary": "#0d1b2a", "--text-secondary": "#415a77", "--accent-blue": "#0077b6", "--accent-green": "#2d9f6f",
      "--accent-purple": "#7c5cbf", "--accent-gold": "#d4960a", "--accent-rose": "#d94e5c",
      "--border-color": "rgba(0, 0, 0, 0.07)", "--error-color": "#d94e5c",
    },
  },
  // ── Pair: Rose (Rose Night / Rose Garden) ──
  {
    id: "dark-rose", name: "Rose", category: "standard", mode: "dark", pairId: "rose",
    preview: { bg: "#1a0f10", fg: "#f0dde0", accent: "#f43f5e", secondary: "#2a1a1c" },
    vars: {
      "--bg-primary": "#1a0f10", "--bg-secondary": "#2a1a1c", "--bg-tertiary": "#3a2528", "--bg-elevated": "#452e32",
      "--text-primary": "#f0dde0", "--text-secondary": "#a88b8e", "--accent-blue": "#f43f5e", "--accent-green": "#059669",
      "--accent-purple": "#a78bfa", "--accent-gold": "#ca8a04", "--accent-rose": "#f43f5e",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f43f5e",
    },
  },
  {
    id: "light-rose", name: "Rose", category: "standard", mode: "light", pairId: "rose",
    preview: { bg: "#fff5f5", fg: "#2d1b1b", accent: "#e11d48", secondary: "#ffe4e6" },
    vars: {
      "--bg-primary": "#fff5f5", "--bg-secondary": "#ffe4e6", "--bg-tertiary": "#fecdd3", "--bg-elevated": "#ffffff",
      "--text-primary": "#2d1b1b", "--text-secondary": "#9f6b6b", "--accent-blue": "#e11d48", "--accent-green": "#059669",
      "--accent-purple": "#8b5cf6", "--accent-gold": "#ca8a04", "--accent-rose": "#e11d48",
      "--border-color": "rgba(0, 0, 0, 0.06)", "--error-color": "#e11d48",
    },
  },
  // ── Pair: High Contrast ──
  {
    id: "hc-dark", name: "High Contrast", category: "high-contrast", mode: "dark", pairId: "hc",
    preview: { bg: "#000000", fg: "#ffffff", accent: "#00e0ff", secondary: "#0a0a0a" },
    vars: {
      "--bg-primary": "#000000", "--bg-secondary": "#0a0a0a", "--bg-tertiary": "#141414", "--bg-elevated": "#1e1e1e",
      "--text-primary": "#ffffff", "--text-secondary": "#cccccc", "--accent-blue": "#00e0ff", "--accent-green": "#00ff88",
      "--accent-purple": "#d0a0ff", "--accent-gold": "#ffdd00", "--accent-rose": "#ff6699",
      "--border-color": "rgba(255, 255, 255, 0.25)", "--error-color": "#ff3333",
    },
  },
  {
    id: "hc-light", name: "High Contrast", category: "high-contrast", mode: "light", pairId: "hc",
    preview: { bg: "#ffffff", fg: "#000000", accent: "#0033cc", secondary: "#f0f0f0" },
    vars: {
      "--bg-primary": "#ffffff", "--bg-secondary": "#f0f0f0", "--bg-tertiary": "#e0e0e0", "--bg-elevated": "#ffffff",
      "--text-primary": "#000000", "--text-secondary": "#333333", "--accent-blue": "#0033cc", "--accent-green": "#006633",
      "--accent-purple": "#6600cc", "--accent-gold": "#996600", "--accent-rose": "#cc0044",
      "--border-color": "rgba(0, 0, 0, 0.3)", "--error-color": "#cc0000",
    },
  },
  // ── Pair: Deuteranopia ──
  {
    id: "cb-deuteranopia-dark", name: "Deuteranopia", category: "color-blind", mode: "dark", pairId: "deuteranopia",
    preview: { bg: "#0f1117", fg: "#e2e4ea", accent: "#648fff", secondary: "#161821" },
    vars: {
      "--bg-primary": "#0f1117", "--bg-secondary": "#161821", "--bg-tertiary": "#1c1f2b", "--bg-elevated": "#222638",
      "--text-primary": "#e2e4ea", "--text-secondary": "#6e7491", "--accent-blue": "#648fff", "--accent-green": "#ffb000",
      "--accent-purple": "#dc267f", "--accent-gold": "#ffb000", "--accent-rose": "#dc267f",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#fe6100",
      "--success-color": "#ffb000", "--warning-color": "#fe6100",
    },
  },
  {
    id: "cb-deuteranopia-light", name: "Deuteranopia", category: "color-blind", mode: "light", pairId: "deuteranopia",
    preview: { bg: "#fafbfd", fg: "#1a1d2e", accent: "#3949ab", secondary: "#f0f1f5" },
    vars: {
      "--bg-primary": "#fafbfd", "--bg-secondary": "#f0f1f5", "--bg-tertiary": "#e6e8ef", "--bg-elevated": "#ffffff",
      "--text-primary": "#1a1d2e", "--text-secondary": "#6b7089", "--accent-blue": "#3949ab", "--accent-green": "#e68a00",
      "--accent-purple": "#ad1457", "--accent-gold": "#e68a00", "--accent-rose": "#ad1457",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#d84315",
      "--success-color": "#e68a00", "--warning-color": "#d84315",
    },
  },
  // ── Pair: Protanopia ──
  {
    id: "cb-protanopia-dark", name: "Protanopia", category: "color-blind", mode: "dark", pairId: "protanopia",
    preview: { bg: "#0f1117", fg: "#e2e4ea", accent: "#785ef0", secondary: "#161821" },
    vars: {
      "--bg-primary": "#0f1117", "--bg-secondary": "#161821", "--bg-tertiary": "#1c1f2b", "--bg-elevated": "#222638",
      "--text-primary": "#e2e4ea", "--text-secondary": "#6e7491", "--accent-blue": "#785ef0", "--accent-green": "#ffb000",
      "--accent-purple": "#648fff", "--accent-gold": "#ffb000", "--accent-rose": "#dc267f",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#fe6100",
      "--success-color": "#ffb000", "--warning-color": "#fe6100",
    },
  },
  {
    id: "cb-protanopia-light", name: "Protanopia", category: "color-blind", mode: "light", pairId: "protanopia",
    preview: { bg: "#fafbfd", fg: "#1a1d2e", accent: "#5c41c9", secondary: "#f0f1f5" },
    vars: {
      "--bg-primary": "#fafbfd", "--bg-secondary": "#f0f1f5", "--bg-tertiary": "#e6e8ef", "--bg-elevated": "#ffffff",
      "--text-primary": "#1a1d2e", "--text-secondary": "#6b7089", "--accent-blue": "#5c41c9", "--accent-green": "#e68a00",
      "--accent-purple": "#3d5afe", "--accent-gold": "#e68a00", "--accent-rose": "#ad1457",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#d84315",
      "--success-color": "#e68a00", "--warning-color": "#d84315",
    },
  },
  // ── Pair: Tritanopia ──
  {
    id: "cb-tritanopia-dark", name: "Tritanopia", category: "color-blind", mode: "dark", pairId: "tritanopia",
    preview: { bg: "#0f1117", fg: "#e2e4ea", accent: "#e8384f", secondary: "#161821" },
    vars: {
      "--bg-primary": "#0f1117", "--bg-secondary": "#161821", "--bg-tertiary": "#1c1f2b", "--bg-elevated": "#222638",
      "--text-primary": "#e2e4ea", "--text-secondary": "#6e7491", "--accent-blue": "#e8384f", "--accent-green": "#37a862",
      "--accent-purple": "#e8384f", "--accent-gold": "#37a862", "--accent-rose": "#e8384f",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#e8384f",
      "--success-color": "#37a862", "--warning-color": "#e8a537",
    },
  },
  {
    id: "cb-tritanopia-light", name: "Tritanopia", category: "color-blind", mode: "light", pairId: "tritanopia",
    preview: { bg: "#fafbfd", fg: "#1a1d2e", accent: "#c62038", secondary: "#f0f1f5" },
    vars: {
      "--bg-primary": "#fafbfd", "--bg-secondary": "#f0f1f5", "--bg-tertiary": "#e6e8ef", "--bg-elevated": "#ffffff",
      "--text-primary": "#1a1d2e", "--text-secondary": "#6b7089", "--accent-blue": "#c62038", "--accent-green": "#2a8a4e",
      "--accent-purple": "#c62038", "--accent-gold": "#2a8a4e", "--accent-rose": "#c62038",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#c62038",
      "--success-color": "#2a8a4e", "--warning-color": "#c69425",
    },
  },

  // ═══════════════════════════════════════════════════════════════════
  //  Popular Developer & Organization Themes
  // ═══════════════════════════════════════════════════════════════════

  // ── Pair: Monokai ──
  {
    id: "dark-monokai", name: "Monokai", category: "standard", mode: "dark", pairId: "monokai",
    preview: { bg: "#272822", fg: "#f8f8f2", accent: "#a6e22e", secondary: "#3e3d32" },
    vars: {
      "--bg-primary": "#272822", "--bg-secondary": "#3e3d32", "--bg-tertiary": "#49483e", "--bg-elevated": "#555449",
      "--text-primary": "#f8f8f2", "--text-secondary": "#a5a08a", "--accent-blue": "#66d9ef", "--accent-green": "#a6e22e",
      "--accent-purple": "#ae81ff", "--accent-gold": "#e6db74", "--accent-rose": "#f92672",
      "--border-color": "rgba(255, 255, 255, 0.07)", "--error-color": "#f92672",
    },
  },
  {
    id: "light-monokai", name: "Monokai", category: "standard", mode: "light", pairId: "monokai",
    preview: { bg: "#fafafa", fg: "#272822", accent: "#629755", secondary: "#eeeee8" },
    vars: {
      "--bg-primary": "#fafafa", "--bg-secondary": "#eeeee8", "--bg-tertiary": "#e0e0d8", "--bg-elevated": "#ffffff",
      "--text-primary": "#272822", "--text-secondary": "#605c46", "--accent-blue": "#1290bf", "--accent-green": "#629755",
      "--accent-purple": "#7a3ea0", "--accent-gold": "#b58900", "--accent-rose": "#c4265e",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#c4265e",
    },
  },
  // ── Pair: Dracula ──
  {
    id: "dark-dracula", name: "Dracula", category: "standard", mode: "dark", pairId: "dracula",
    preview: { bg: "#282a36", fg: "#f8f8f2", accent: "#bd93f9", secondary: "#44475a" },
    vars: {
      "--bg-primary": "#282a36", "--bg-secondary": "#44475a", "--bg-tertiary": "#4e5166", "--bg-elevated": "#555770",
      "--text-primary": "#f8f8f2", "--text-secondary": "#8a96c0", "--accent-blue": "#8be9fd", "--accent-green": "#50fa7b",
      "--accent-purple": "#bd93f9", "--accent-gold": "#f1fa8c", "--accent-rose": "#ff79c6",
      "--border-color": "rgba(255, 255, 255, 0.08)", "--error-color": "#ff5555",
    },
  },
  {
    id: "light-dracula", name: "Dracula", category: "standard", mode: "light", pairId: "dracula",
    preview: { bg: "#f8f8f2", fg: "#282a36", accent: "#7c3aed", secondary: "#ededec" },
    vars: {
      "--bg-primary": "#f8f8f2", "--bg-secondary": "#ededec", "--bg-tertiary": "#e0dfe0", "--bg-elevated": "#ffffff",
      "--text-primary": "#282a36", "--text-secondary": "#4e5a7e", "--accent-blue": "#0891b2", "--accent-green": "#16a34a",
      "--accent-purple": "#7c3aed", "--accent-gold": "#a16207", "--accent-rose": "#db2777",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#dc2626",
    },
  },
  // ── Pair: Nord ──
  {
    id: "dark-nord", name: "Nord", category: "standard", mode: "dark", pairId: "nord",
    preview: { bg: "#2e3440", fg: "#eceff4", accent: "#88c0d0", secondary: "#3b4252" },
    vars: {
      "--bg-primary": "#2e3440", "--bg-secondary": "#3b4252", "--bg-tertiary": "#434c5e", "--bg-elevated": "#4c566a",
      "--text-primary": "#eceff4", "--text-secondary": "#9aa4b8", "--accent-blue": "#88c0d0", "--accent-green": "#a3be8c",
      "--accent-purple": "#b48ead", "--accent-gold": "#ebcb8b", "--accent-rose": "#bf616a",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#bf616a",
    },
  },
  {
    id: "light-nord", name: "Nord", category: "standard", mode: "light", pairId: "nord",
    preview: { bg: "#eceff4", fg: "#2e3440", accent: "#5e81ac", secondary: "#e5e9f0" },
    vars: {
      "--bg-primary": "#eceff4", "--bg-secondary": "#e5e9f0", "--bg-tertiary": "#d8dee9", "--bg-elevated": "#f8fafc",
      "--text-primary": "#2e3440", "--text-secondary": "#4c566a", "--accent-blue": "#5e81ac", "--accent-green": "#689d6a",
      "--accent-purple": "#8f6594", "--accent-gold": "#c08b30", "--accent-rose": "#a3373e",
      "--border-color": "rgba(0, 0, 0, 0.07)", "--error-color": "#a3373e",
    },
  },
  // ── Pair: One (Atom) ──
  {
    id: "dark-one", name: "One", category: "standard", mode: "dark", pairId: "one",
    preview: { bg: "#282c34", fg: "#abb2bf", accent: "#61afef", secondary: "#21252b" },
    vars: {
      "--bg-primary": "#282c34", "--bg-secondary": "#21252b", "--bg-tertiary": "#2c313a", "--bg-elevated": "#333842",
      "--text-primary": "#abb2bf", "--text-secondary": "#838994", "--accent-blue": "#61afef", "--accent-green": "#98c379",
      "--accent-purple": "#c678dd", "--accent-gold": "#e5c07b", "--accent-rose": "#e06c75",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#e06c75",
    },
  },
  {
    id: "light-one", name: "One", category: "standard", mode: "light", pairId: "one",
    preview: { bg: "#fafafa", fg: "#383a42", accent: "#4078f2", secondary: "#f0f0f0" },
    vars: {
      "--bg-primary": "#fafafa", "--bg-secondary": "#f0f0f0", "--bg-tertiary": "#e5e5e6", "--bg-elevated": "#ffffff",
      "--text-primary": "#383a42", "--text-secondary": "#696a70", "--accent-blue": "#4078f2", "--accent-green": "#50a14f",
      "--accent-purple": "#a626a4", "--accent-gold": "#c18401", "--accent-rose": "#e45649",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#e45649",
    },
  },
  // ── Pair: GitHub ──
  {
    id: "dark-github", name: "GitHub", category: "standard", mode: "dark", pairId: "github",
    preview: { bg: "#0d1117", fg: "#e6edf3", accent: "#58a6ff", secondary: "#161b22" },
    vars: {
      "--bg-primary": "#0d1117", "--bg-secondary": "#161b22", "--bg-tertiary": "#21262d", "--bg-elevated": "#30363d",
      "--text-primary": "#e6edf3", "--text-secondary": "#8b929a", "--accent-blue": "#58a6ff", "--accent-green": "#3fb950",
      "--accent-purple": "#bc8cff", "--accent-gold": "#d29922", "--accent-rose": "#f85149",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f85149",
    },
  },
  {
    id: "light-github", name: "GitHub", category: "standard", mode: "light", pairId: "github",
    preview: { bg: "#ffffff", fg: "#1f2328", accent: "#0969da", secondary: "#f6f8fa" },
    vars: {
      "--bg-primary": "#ffffff", "--bg-secondary": "#f6f8fa", "--bg-tertiary": "#eaeef2", "--bg-elevated": "#ffffff",
      "--text-primary": "#1f2328", "--text-secondary": "#656d76", "--accent-blue": "#0969da", "--accent-green": "#1a7f37",
      "--accent-purple": "#8250df", "--accent-gold": "#9a6700", "--accent-rose": "#cf222e",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#cf222e",
    },
  },
  // ── Pair: Catppuccin (Mocha/Latte) ──
  {
    id: "dark-catppuccin", name: "Catppuccin", category: "standard", mode: "dark", pairId: "catppuccin",
    preview: { bg: "#1e1e2e", fg: "#cdd6f4", accent: "#89b4fa", secondary: "#313244" },
    vars: {
      "--bg-primary": "#1e1e2e", "--bg-secondary": "#313244", "--bg-tertiary": "#45475a", "--bg-elevated": "#585b70",
      "--text-primary": "#cdd6f4", "--text-secondary": "#9399b2", "--accent-blue": "#89b4fa", "--accent-green": "#a6e3a1",
      "--accent-purple": "#cba6f7", "--accent-gold": "#f9e2af", "--accent-rose": "#f38ba8",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f38ba8",
    },
  },
  {
    id: "light-catppuccin", name: "Catppuccin", category: "standard", mode: "light", pairId: "catppuccin",
    preview: { bg: "#eff1f5", fg: "#4c4f69", accent: "#1e66f5", secondary: "#e6e9ef" },
    vars: {
      "--bg-primary": "#eff1f5", "--bg-secondary": "#e6e9ef", "--bg-tertiary": "#ccd0da", "--bg-elevated": "#ffffff",
      "--text-primary": "#4c4f69", "--text-secondary": "#5c5f73", "--accent-blue": "#1e66f5", "--accent-green": "#40a02b",
      "--accent-purple": "#8839ef", "--accent-gold": "#df8e1d", "--accent-rose": "#d20f39",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#d20f39",
    },
  },
  // ── Pair: Gruvbox ──
  {
    id: "dark-gruvbox", name: "Gruvbox", category: "standard", mode: "dark", pairId: "gruvbox",
    preview: { bg: "#282828", fg: "#ebdbb2", accent: "#fabd2f", secondary: "#3c3836" },
    vars: {
      "--bg-primary": "#282828", "--bg-secondary": "#3c3836", "--bg-tertiary": "#504945", "--bg-elevated": "#665c54",
      "--text-primary": "#ebdbb2", "--text-secondary": "#a89b8c", "--accent-blue": "#83a598", "--accent-green": "#b8bb26",
      "--accent-purple": "#d3869b", "--accent-gold": "#fabd2f", "--accent-rose": "#fb4934",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#fb4934",
    },
  },
  {
    id: "light-gruvbox", name: "Gruvbox", category: "standard", mode: "light", pairId: "gruvbox",
    preview: { bg: "#fbf1c7", fg: "#3c3836", accent: "#b57614", secondary: "#ebdbb2" },
    vars: {
      "--bg-primary": "#fbf1c7", "--bg-secondary": "#ebdbb2", "--bg-tertiary": "#d5c4a1", "--bg-elevated": "#fffbef",
      "--text-primary": "#3c3836", "--text-secondary": "#5e5448", "--accent-blue": "#076678", "--accent-green": "#79740e",
      "--accent-purple": "#8f3f71", "--accent-gold": "#b57614", "--accent-rose": "#9d0006",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#9d0006",
    },
  },
  // ── Pair: Tokyo Night ──
  {
    id: "dark-tokyo", name: "Tokyo", category: "standard", mode: "dark", pairId: "tokyo",
    preview: { bg: "#1a1b26", fg: "#c0caf5", accent: "#7aa2f7", secondary: "#24283b" },
    vars: {
      "--bg-primary": "#1a1b26", "--bg-secondary": "#24283b", "--bg-tertiary": "#2f3347", "--bg-elevated": "#3b3f54",
      "--text-primary": "#c0caf5", "--text-secondary": "#7a82a8", "--accent-blue": "#7aa2f7", "--accent-green": "#9ece6a",
      "--accent-purple": "#bb9af7", "--accent-gold": "#e0af68", "--accent-rose": "#f7768e",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f7768e",
    },
  },
  {
    id: "light-tokyo", name: "Tokyo", category: "standard", mode: "light", pairId: "tokyo",
    preview: { bg: "#d5d6db", fg: "#343b58", accent: "#34548a", secondary: "#c8c8ce" },
    vars: {
      "--bg-primary": "#d5d6db", "--bg-secondary": "#c8c8ce", "--bg-tertiary": "#b8b8c0", "--bg-elevated": "#e5e5ea",
      "--text-primary": "#343b58", "--text-secondary": "#4a5880", "--accent-blue": "#34548a", "--accent-green": "#485e30",
      "--accent-purple": "#7847bd", "--accent-gold": "#8f5e15", "--accent-rose": "#8c4351",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#8c4351",
    },
  },
  // ── Pair: Material ──
  {
    id: "dark-material", name: "Material", category: "standard", mode: "dark", pairId: "material",
    preview: { bg: "#212121", fg: "#eeffff", accent: "#82aaff", secondary: "#303030" },
    vars: {
      "--bg-primary": "#212121", "--bg-secondary": "#303030", "--bg-tertiary": "#3a3a3a", "--bg-elevated": "#424242",
      "--text-primary": "#eeffff", "--text-secondary": "#8a8a8a", "--accent-blue": "#82aaff", "--accent-green": "#c3e88d",
      "--accent-purple": "#c792ea", "--accent-gold": "#ffcb6b", "--accent-rose": "#f07178",
      "--border-color": "rgba(255, 255, 255, 0.05)", "--error-color": "#f07178",
    },
  },
  {
    id: "light-material", name: "Material", category: "standard", mode: "light", pairId: "material",
    preview: { bg: "#fafafa", fg: "#546e7a", accent: "#6182b8", secondary: "#eaeaea" },
    vars: {
      "--bg-primary": "#fafafa", "--bg-secondary": "#eaeaea", "--bg-tertiary": "#d4d4d4", "--bg-elevated": "#ffffff",
      "--text-primary": "#546e7a", "--text-secondary": "#5e7680", "--accent-blue": "#6182b8", "--accent-green": "#91b859",
      "--accent-purple": "#7c4dff", "--accent-gold": "#f6a434", "--accent-rose": "#e53935",
      "--border-color": "rgba(0, 0, 0, 0.07)", "--error-color": "#e53935",
    },
  },
  // ── Pair: Solarized ──
  {
    id: "dark-solarized", name: "Solarized", category: "standard", mode: "dark", pairId: "solarized",
    preview: { bg: "#002b36", fg: "#839496", accent: "#268bd2", secondary: "#073642" },
    vars: {
      "--bg-primary": "#002b36", "--bg-secondary": "#073642", "--bg-tertiary": "#0a4050", "--bg-elevated": "#0d4f5e",
      "--text-primary": "#93a1a1", "--text-secondary": "#6d8388", "--accent-blue": "#268bd2", "--accent-green": "#859900",
      "--accent-purple": "#6c71c4", "--accent-gold": "#b58900", "--accent-rose": "#dc322f",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#dc322f",
    },
  },
  {
    id: "light-solarized", name: "Solarized", category: "standard", mode: "light", pairId: "solarized",
    preview: { bg: "#fdf6e3", fg: "#4a5a60", accent: "#268bd2", secondary: "#eee8d5" },
    vars: {
      "--bg-primary": "#fdf6e3", "--bg-secondary": "#eee8d5", "--bg-tertiary": "#e0dbc7", "--bg-elevated": "#fffdf5",
      "--text-primary": "#4a5a60", "--text-secondary": "#6b7c80", "--accent-blue": "#268bd2", "--accent-green": "#859900",
      "--accent-purple": "#6c71c4", "--accent-gold": "#b58900", "--accent-rose": "#dc322f",
      "--border-color": "rgba(0, 0, 0, 0.07)", "--error-color": "#dc322f",
    },
  },
  // ── Pair: Palenight ──
  {
    id: "dark-palenight", name: "Palenight", category: "standard", mode: "dark", pairId: "palenight",
    preview: { bg: "#292d3e", fg: "#a6accd", accent: "#82aaff", secondary: "#34324a" },
    vars: {
      "--bg-primary": "#292d3e", "--bg-secondary": "#34324a", "--bg-tertiary": "#3e3c56", "--bg-elevated": "#484660",
      "--text-primary": "#bfc5e0", "--text-secondary": "#8088b0", "--accent-blue": "#82aaff", "--accent-green": "#c3e88d",
      "--accent-purple": "#c792ea", "--accent-gold": "#ffcb6b", "--accent-rose": "#f07178",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f07178",
    },
  },
  {
    id: "light-palenight", name: "Palenight", category: "standard", mode: "light", pairId: "palenight",
    preview: { bg: "#f0f0f8", fg: "#3b3d55", accent: "#5a6acf", secondary: "#e4e4ef" },
    vars: {
      "--bg-primary": "#f0f0f8", "--bg-secondary": "#e4e4ef", "--bg-tertiary": "#d4d4e2", "--bg-elevated": "#fafaff",
      "--text-primary": "#3b3d55", "--text-secondary": "#555b7a", "--accent-blue": "#5a6acf", "--accent-green": "#689d6a",
      "--accent-purple": "#9c5fb5", "--accent-gold": "#c08b30", "--accent-rose": "#c45060",
      "--border-color": "rgba(0, 0, 0, 0.07)", "--error-color": "#c45060",
    },
  },
  // ── Pair: Ayu ──
  {
    id: "dark-ayu", name: "Ayu", category: "standard", mode: "dark", pairId: "ayu",
    preview: { bg: "#0a0e14", fg: "#b3b1ad", accent: "#ffb454", secondary: "#1f2430" },
    vars: {
      "--bg-primary": "#0a0e14", "--bg-secondary": "#1f2430", "--bg-tertiary": "#272d38", "--bg-elevated": "#2e3440",
      "--text-primary": "#b3b1ad", "--text-secondary": "#7e8894", "--accent-blue": "#36a3d9", "--accent-green": "#bae67e",
      "--accent-purple": "#d4bfff", "--accent-gold": "#ffb454", "--accent-rose": "#ff3333",
      "--border-color": "rgba(255, 255, 255, 0.05)", "--error-color": "#ff3333",
    },
  },
  {
    id: "light-ayu", name: "Ayu", category: "standard", mode: "light", pairId: "ayu",
    preview: { bg: "#fafafa", fg: "#575f66", accent: "#ff9940", secondary: "#f0f0f0" },
    vars: {
      "--bg-primary": "#fafafa", "--bg-secondary": "#f0f0f0", "--bg-tertiary": "#e1e1e1", "--bg-elevated": "#ffffff",
      "--text-primary": "#575f66", "--text-secondary": "#6e7478", "--accent-blue": "#399ee6", "--accent-green": "#86b300",
      "--accent-purple": "#a37acc", "--accent-gold": "#ff9940", "--accent-rose": "#f51818",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#f51818",
    },
  },
  // ── Pair: Slack (Organization) ──
  {
    id: "dark-slack", name: "Slack", category: "standard", mode: "dark", pairId: "slack",
    preview: { bg: "#1a1d21", fg: "#d1d2d3", accent: "#36c5f0", secondary: "#27242c" },
    vars: {
      "--bg-primary": "#1a1d21", "--bg-secondary": "#27242c", "--bg-tertiary": "#332f3b", "--bg-elevated": "#3d3848",
      "--text-primary": "#d1d2d3", "--text-secondary": "#9a9a9d", "--accent-blue": "#36c5f0", "--accent-green": "#2eb67d",
      "--accent-purple": "#611f69", "--accent-gold": "#ecb22e", "--accent-rose": "#e01e5a",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#e01e5a",
    },
  },
  {
    id: "light-slack", name: "Slack", category: "standard", mode: "light", pairId: "slack",
    preview: { bg: "#ffffff", fg: "#1d1c1d", accent: "#1264a3", secondary: "#f8f8f8" },
    vars: {
      "--bg-primary": "#ffffff", "--bg-secondary": "#f8f8f8", "--bg-tertiary": "#ececec", "--bg-elevated": "#ffffff",
      "--text-primary": "#1d1c1d", "--text-secondary": "#616061", "--accent-blue": "#1264a3", "--accent-green": "#007a5a",
      "--accent-purple": "#611f69", "--accent-gold": "#daa520", "--accent-rose": "#e01e5a",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#e01e5a",
    },
  },
  // ── Pair: Cobalt ──
  {
    id: "dark-cobalt", name: "Cobalt", category: "standard", mode: "dark", pairId: "cobalt",
    preview: { bg: "#193549", fg: "#e1efff", accent: "#ffc600", secondary: "#1f4662" },
    vars: {
      "--bg-primary": "#193549", "--bg-secondary": "#1f4662", "--bg-tertiary": "#245170", "--bg-elevated": "#2a5c80",
      "--text-primary": "#e1efff", "--text-secondary": "#6fa0c7", "--accent-blue": "#80ffbb", "--accent-green": "#3ad900",
      "--accent-purple": "#fb94ff", "--accent-gold": "#ffc600", "--accent-rose": "#ff628c",
      "--border-color": "rgba(255, 255, 255, 0.08)", "--error-color": "#ff628c",
    },
  },
  {
    id: "light-cobalt", name: "Cobalt", category: "standard", mode: "light", pairId: "cobalt",
    preview: { bg: "#f0f5fa", fg: "#193549", accent: "#b8860b", secondary: "#dfe8f0" },
    vars: {
      "--bg-primary": "#f0f5fa", "--bg-secondary": "#dfe8f0", "--bg-tertiary": "#c8d8e4", "--bg-elevated": "#ffffff",
      "--text-primary": "#193549", "--text-secondary": "#4e7a97", "--accent-blue": "#16825d", "--accent-green": "#2e8b00",
      "--accent-purple": "#9b42a0", "--accent-gold": "#b8860b", "--accent-rose": "#c0284a",
      "--border-color": "rgba(0, 0, 0, 0.07)", "--error-color": "#c0284a",
    },
  },
  // ── Pair: Synthwave ──
  {
    id: "dark-synthwave", name: "Synthwave '84", category: "standard", mode: "dark", pairId: "synthwave",
    preview: { bg: "#262335", fg: "#e0def4", accent: "#f97e72", secondary: "#34294f" },
    vars: {
      "--bg-primary": "#262335", "--bg-secondary": "#34294f", "--bg-tertiary": "#3e3461", "--bg-elevated": "#4a3f73",
      "--text-primary": "#e0def4", "--text-secondary": "#9d98b8", "--accent-blue": "#72f1b8", "--accent-green": "#72f1b8",
      "--accent-purple": "#f97e72", "--accent-gold": "#fede5d", "--accent-rose": "#fe4450",
      "--border-color": "rgba(255, 255, 255, 0.07)", "--error-color": "#fe4450",
    },
  },
  {
    id: "light-synthwave", name: "Synthwave '84", category: "standard", mode: "light", pairId: "synthwave",
    preview: { bg: "#f5f0ff", fg: "#2d2350", accent: "#c44040", secondary: "#e8e0f5" },
    vars: {
      "--bg-primary": "#f5f0ff", "--bg-secondary": "#e8e0f5", "--bg-tertiary": "#d8cee8", "--bg-elevated": "#ffffff",
      "--text-primary": "#2d2350", "--text-secondary": "#504068", "--accent-blue": "#2a8a5e", "--accent-green": "#2a8a5e",
      "--accent-purple": "#c44040", "--accent-gold": "#a78000", "--accent-rose": "#c02030",
      "--border-color": "rgba(0, 0, 0, 0.07)", "--error-color": "#c02030",
    },
  },
  // ── Pair: Everforest ──
  {
    id: "dark-everforest", name: "Everforest", category: "standard", mode: "dark", pairId: "everforest",
    preview: { bg: "#2d353b", fg: "#d3c6aa", accent: "#a7c080", secondary: "#343f44" },
    vars: {
      "--bg-primary": "#2d353b", "--bg-secondary": "#343f44", "--bg-tertiary": "#3d484d", "--bg-elevated": "#475258",
      "--text-primary": "#d3c6aa", "--text-secondary": "#859289", "--accent-blue": "#7fbbb3", "--accent-green": "#a7c080",
      "--accent-purple": "#d699b6", "--accent-gold": "#dbbc7f", "--accent-rose": "#e67e80",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#e67e80",
    },
  },
  {
    id: "light-everforest", name: "Everforest", category: "standard", mode: "light", pairId: "everforest",
    preview: { bg: "#fdf6e3", fg: "#5c6a72", accent: "#8da101", secondary: "#f0ead2" },
    vars: {
      "--bg-primary": "#fdf6e3", "--bg-secondary": "#f0ead2", "--bg-tertiary": "#e0dab8", "--bg-elevated": "#fffbf0",
      "--text-primary": "#5c6a72", "--text-secondary": "#5c6860", "--accent-blue": "#3a94c5", "--accent-green": "#8da101",
      "--accent-purple": "#df69ba", "--accent-gold": "#dfa000", "--accent-rose": "#f85552",
      "--border-color": "rgba(0, 0, 0, 0.07)", "--error-color": "#f85552",
    },
  },
  // ── Pair: Kanagawa ──
  {
    id: "dark-kanagawa", name: "Kanagawa", category: "standard", mode: "dark", pairId: "kanagawa",
    preview: { bg: "#1f1f28", fg: "#dcd7ba", accent: "#7e9cd8", secondary: "#2a2a37" },
    vars: {
      "--bg-primary": "#1f1f28", "--bg-secondary": "#2a2a37", "--bg-tertiary": "#363646", "--bg-elevated": "#3d3d55",
      "--text-primary": "#dcd7ba", "--text-secondary": "#908f85", "--accent-blue": "#7e9cd8", "--accent-green": "#98bb6c",
      "--accent-purple": "#957fb8", "--accent-gold": "#e6c384", "--accent-rose": "#e82424",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#e82424",
    },
  },
  {
    id: "light-kanagawa", name: "Kanagawa", category: "standard", mode: "light", pairId: "kanagawa",
    preview: { bg: "#f2ecbc", fg: "#43436c", accent: "#4d699b", secondary: "#e7dba0" },
    vars: {
      "--bg-primary": "#f2ecbc", "--bg-secondary": "#e7dba0", "--bg-tertiary": "#d8cc88", "--bg-elevated": "#faf5d0",
      "--text-primary": "#43436c", "--text-secondary": "#5e5e50", "--accent-blue": "#4d699b", "--accent-green": "#6f894e",
      "--accent-purple": "#624c83", "--accent-gold": "#a96b2c", "--accent-rose": "#c84053",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#c84053",
    },
  },

  // ── Pair: Sherwood (Fintech Green) ──
  {
    id: "dark-sherwood", name: "Sherwood", category: "standard", mode: "dark", pairId: "sherwood",
    preview: { bg: "#0a0a0a", fg: "#f0f0f0", accent: "#00c805", secondary: "#141414" },
    vars: {
      "--bg-primary": "#0a0a0a", "--bg-secondary": "#141414", "--bg-tertiary": "#1c1c1c", "--bg-elevated": "#242424",
      "--text-primary": "#f0f0f0", "--text-secondary": "#8a8a8a", "--accent-blue": "#00c805", "--accent-green": "#00c805",
      "--accent-purple": "#9c88ff", "--accent-gold": "#f0c040", "--accent-rose": "#ff4d4d",
      "--border-color": "rgba(255, 255, 255, 0.07)", "--error-color": "#ff4d4d",
      "--success-color": "#00c805", "--warning-color": "#f0c040",
    },
  },
  {
    id: "light-sherwood", name: "Sherwood", category: "standard", mode: "light", pairId: "sherwood",
    preview: { bg: "#ffffff", fg: "#1a1a1a", accent: "#00c805", secondary: "#f5f5f5" },
    vars: {
      "--bg-primary": "#ffffff", "--bg-secondary": "#f5f5f5", "--bg-tertiary": "#ebebeb", "--bg-elevated": "#ffffff",
      "--text-primary": "#1a1a1a", "--text-secondary": "#6e6e6e", "--accent-blue": "#00a804", "--accent-green": "#00a804",
      "--accent-purple": "#6c5ce7", "--accent-gold": "#c8960a", "--accent-rose": "#e03030",
      "--border-color": "rgba(0, 0, 0, 0.09)", "--error-color": "#e03030",
      "--success-color": "#00a804", "--warning-color": "#c8960a",
    },
  },

  // ── Pair: Blaze (Red · Black · Gold) ──
  {
    id: "dark-blaze", name: "Blaze", category: "standard", mode: "dark", pairId: "blaze",
    preview: { bg: "#0c0806", fg: "#f5e8e0", accent: "#e84428", secondary: "#1c1210" },
    vars: {
      "--bg-primary": "#0c0806", "--bg-secondary": "#1c1210", "--bg-tertiary": "#2a1c18", "--bg-elevated": "#362420",
      "--text-primary": "#f5e8e0", "--text-secondary": "#a08878", "--accent-blue": "#e84428", "--accent-green": "#ffe44d",
      "--accent-purple": "#d44030", "--accent-gold": "#ffe44d", "--accent-rose": "#e84428",
      "--border-color": "rgba(232, 68, 40, 0.14)", "--error-color": "#ff5040",
      "--success-color": "#ffe44d", "--warning-color": "#f0a020", "--info-color": "#e84428",
      "--accent-color": "#e84428", "--glow-accent": "0 0 20px rgba(232, 68, 40, 0.25)",
    },
  },
  {
    id: "light-blaze", name: "Blaze", category: "standard", mode: "light", pairId: "blaze",
    preview: { bg: "#fef8f5", fg: "#1a0c08", accent: "#d44030", secondary: "#f8e8e2" },
    vars: {
      "--bg-primary": "#fef8f5", "--bg-secondary": "#f8e8e2", "--bg-tertiary": "#f0d8cc", "--bg-elevated": "#ffffff",
      "--text-primary": "#1a0c08", "--text-secondary": "#7a5848", "--accent-blue": "#d44030", "--accent-green": "#c8a010",
      "--accent-purple": "#b83020", "--accent-gold": "#c8a010", "--accent-rose": "#d44030",
      "--border-color": "rgba(212, 64, 48, 0.12)", "--error-color": "#b82020",
      "--success-color": "#c8a010", "--warning-color": "#d08818", "--info-color": "#d44030",
      "--accent-color": "#d44030", "--glow-accent": "0 0 20px rgba(212, 64, 48, 0.15)",
    },
  },
  // ── Pair: Circuit (Yellow · Green) — racing-flag palette background
  // started as the exact Ferrari-green swatch #039B4E at its native, quite
  // bright lightness; on a later request it reads as that same green "in
  // shade" instead — same hue (~150°) and saturation, background steps
  // scaled to ~55% of their original lightness (L 31%→17% on the primary,
  // down through the secondary/tertiary/elevated steps in proportion) so
  // the ramp keeps its shape, just darker. The hero accent is a sampled
  // golden yellow (#E6D514, distinct from the flatter lemon #FFF200
  // already in the gold slot) swapped in for the original vivid red hero,
  // and reads brighter against the darker green. `error-color` stays red
  // (`#ED1C24`) on purpose — that's a semantic signal color, not part of
  // the theme's identity, and every other pair in this file keeps it red
  // regardless of hero hue. Named for the motorsport palette itself, not
  // any car marque — see the "Remove brand-named themes" commit, which
  // pulled a Ferrari-named pair for the same reason. Distinct from Blaze
  // (red/black/gold, no green) above. ──
  {
    id: "dark-circuit", name: "Circuit", category: "standard", mode: "dark", pairId: "circuit",
    preview: { bg: "#02552b", fg: "#ffffff", accent: "#e6d514", secondary: "#0b5c36" },
    vars: {
      "--bg-primary": "#02552b", "--bg-secondary": "#0b5c36", "--bg-tertiary": "#13633e", "--bg-elevated": "#1a6a46",
      "--text-primary": "#ffffff", "--text-secondary": "#d6f5e3", "--accent-blue": "#e6d514", "--accent-green": "#6dffb0",
      "--accent-purple": "#b8151b", "--accent-gold": "#fff200", "--accent-rose": "#ff4d54",
      "--border-color": "rgba(255, 255, 255, 0.18)", "--error-color": "#ed1c24",
    },
  },
  {
    id: "light-circuit", name: "Circuit", category: "standard", mode: "light", pairId: "circuit",
    preview: { bg: "#fef8f2", fg: "#1a0c08", accent: "#b39a00", secondary: "#f8ece0" },
    vars: {
      "--bg-primary": "#fef8f2", "--bg-secondary": "#f8ece0", "--bg-tertiary": "#f0dcc8", "--bg-elevated": "#ffffff",
      "--text-primary": "#1a0c08", "--text-secondary": "#7a5848", "--accent-blue": "#b39a00", "--accent-green": "#00753b",
      "--accent-purple": "#8a0f14", "--accent-gold": "#b8a600", "--accent-rose": "#d4272e",
      "--border-color": "rgba(179, 154, 0, 0.14)", "--error-color": "#c8141a",
    },
  },
  // ── Pair: Marshal (Yellow · Red) — Circuit's racing-flag sibling: a flag
  // marshal's stop/caution pair instead of Circuit's green. Background is
  // the exact Ferrari-red swatch #E42528 (brandcolorcode.com/ferrari),
  // shaded the same way Circuit's green was — same hue (~359°) and native
  // saturation, background steps scaled to ~55% of their original lightness
  // (L 52%→28.5% on the primary) so it reads as that red in shade rather
  // than under a floodlight. The hero accent is a sampled bottle-yellow
  // (#F7D000, distinct from the flat lemon #FFF200 already in the gold
  // slot) and stays at its native brightness — only the background is
  // shaded, so the hero keeps popping against it. `accent-green` holds a
  // warm gold rather than literal green, matching Blaze's convention for
  // themes with no green in their identity. `error-color` stays the file's
  // standard red (`#ED1C24`) on purpose, same reasoning as Circuit. Named
  // for the flag signal, not any car marque — see the "Remove brand-named
  // themes" commit. ──
  {
    id: "dark-marshal", name: "Marshal", category: "standard", mode: "dark", pairId: "marshal",
    preview: { bg: "#811012", fg: "#ffffff", accent: "#f7d000", secondary: "#841d1f" },
    vars: {
      "--bg-primary": "#811012", "--bg-secondary": "#841d1f", "--bg-tertiary": "#892729", "--bg-elevated": "#8e3132",
      "--text-primary": "#ffffff", "--text-secondary": "#f6dfe0", "--accent-blue": "#f7d000", "--accent-green": "#ffe44d",
      "--accent-purple": "#c9660a", "--accent-gold": "#fff200", "--accent-rose": "#ff4d8a",
      "--border-color": "rgba(247, 208, 0, 0.18)", "--error-color": "#ed1c24",
    },
  },
  {
    id: "light-marshal", name: "Marshal", category: "standard", mode: "light", pairId: "marshal",
    preview: { bg: "#fef8f2", fg: "#1a0c08", accent: "#b19500", secondary: "#f8ece0" },
    vars: {
      "--bg-primary": "#fef8f2", "--bg-secondary": "#f8ece0", "--bg-tertiary": "#f0dcc8", "--bg-elevated": "#ffffff",
      "--text-primary": "#1a0c08", "--text-secondary": "#7a5848", "--accent-blue": "#b19500", "--accent-green": "#c8a010",
      "--accent-purple": "#974900", "--accent-gold": "#b8a600", "--accent-rose": "#ee0051",
      "--border-color": "rgba(177, 149, 0, 0.14)", "--error-color": "#c8141a",
    },
  },
  // ── Pair: Celeste (Yellow · Azzurro La Plata) — the third racing-flag
  // sibling, national-racing-color blue instead of Circuit's green or
  // Marshal's red: the pale Argentine "Azzurro La Plata" Fangio's Maserati
  // 250F carried (no single branded hex is published for it the way
  // Ferrari's is — this uses the commonly cited Argentine celeste,
  // #75AADB), shaded the same way as its siblings — same hue (~209°) and
  // native saturation, background steps scaled to ~55% of their original
  // lightness (L 66%→36% on the primary). It lands lighter than Circuit's
  // or Marshal's shaded primaries even after the same scaling, because the
  // source blue itself is a pale one — a shaded pale blue is a muted steel,
  // not a near-navy, and that is in character for the color. The hero
  // accent is the same sampled bottle-yellow (#F7D000) as Marshal — Fangio's
  // 250F carried a yellow band on the Azzurro La Plata body, so the same
  // hero reads as historically apt here rather than reused for convenience.
  // `accent-purple`/border reuse Marshal's exact burnt-orange and yellow
  // values so the three racing-flag pairs share a family palette rather
  // than drifting. `error-color` stays the file's standard red (`#ED1C24`),
  // same reasoning as its siblings. Named for the color itself (Spanish/
  // Italian for sky-blue), not any driver or marque. ──
  {
    id: "dark-celeste", name: "Celeste", category: "standard", mode: "dark", pairId: "celeste",
    preview: { bg: "#265e92", fg: "#ffffff", accent: "#f7d000", secondary: "#346694" },
    vars: {
      "--bg-primary": "#265e92", "--bg-secondary": "#346694", "--bg-tertiary": "#3f6d99", "--bg-elevated": "#49759d",
      "--text-primary": "#ffffff", "--text-secondary": "#dfebf6", "--accent-blue": "#f7d000", "--accent-green": "#ffe44d",
      "--accent-purple": "#c9660a", "--accent-gold": "#fff200", "--accent-rose": "#ff6b6b",
      "--border-color": "rgba(247, 208, 0, 0.18)", "--error-color": "#ed1c24",
    },
  },
  {
    id: "light-celeste", name: "Celeste", category: "standard", mode: "light", pairId: "celeste",
    preview: { bg: "#fef8f2", fg: "#1a0c08", accent: "#b19500", secondary: "#f8ece0" },
    vars: {
      "--bg-primary": "#fef8f2", "--bg-secondary": "#f8ece0", "--bg-tertiary": "#f0dcc8", "--bg-elevated": "#ffffff",
      "--text-primary": "#1a0c08", "--text-secondary": "#7a5848", "--accent-blue": "#b19500", "--accent-green": "#c8a010",
      "--accent-purple": "#974900", "--accent-gold": "#b8a600", "--accent-rose": "#d42e2e",
      "--border-color": "rgba(177, 149, 0, 0.14)", "--error-color": "#c8141a",
    },
  },
  // ── Pair: Giallo (Black · Giallo Luce) — the fourth racing-flag sibling,
  // and the first to put yellow in the background slot instead of the hero
  // slot Circuit/Marshal/Celeste all gave it. No branded hex is published
  // for "Giallo Luce" ("light/luminous yellow") either, so this uses a
  // clean, bright yellow (#F5E400) shaded the same ~55%-lightness way as
  // its siblings (L 48%→26% on the primary) — it lands as a dark olive-
  // mustard, not a mellow gold, because shading a fully saturated yellow
  // this far pulls it toward olive rather than toward brown the way a red
  // or green does. The hero flips to black (#161616) — the checkered-flag
  // half of a racing-flag palette, and the one hero color that doesn't
  // collide with a yellow-family background the way another yellow would.
  // Because a near-black hero has little lightness gap from the shaded
  // background to read against, `border-color` breaks from the sibling
  // convention of tinting from the hero and uses a plain white overlay in
  // dark mode (black in light mode) instead — the same fallback Circuit
  // used before it had a hero color worth tinting from. `accent-purple`/
  // `accent-gold` reuse the burnt-orange/lemon constants shared by the
  // other three siblings, and `error-color` stays the file's standard red,
  // same reasoning as the rest of the family. Named for the color itself,
  // not any marque. ──
  {
    id: "dark-giallo", name: "Giallo", category: "standard", mode: "dark", pairId: "giallo",
    preview: { bg: "#867d00", fg: "#ffffff", accent: "#161616", secondary: "#89800e" },
    vars: {
      "--bg-primary": "#867d00", "--bg-secondary": "#89800e", "--bg-tertiary": "#8e8518", "--bg-elevated": "#938b22",
      "--text-primary": "#ffffff", "--text-secondary": "#f6f4df", "--accent-blue": "#161616", "--accent-green": "#ffe44d",
      "--accent-purple": "#c9660a", "--accent-gold": "#fff200", "--accent-rose": "#ff6b6b",
      "--border-color": "rgba(255, 255, 255, 0.18)", "--error-color": "#ed1c24",
    },
  },
  {
    id: "light-giallo", name: "Giallo", category: "standard", mode: "light", pairId: "giallo",
    preview: { bg: "#fef8f2", fg: "#1a0c08", accent: "#161616", secondary: "#f8ece0" },
    vars: {
      "--bg-primary": "#fef8f2", "--bg-secondary": "#f8ece0", "--bg-tertiary": "#f0dcc8", "--bg-elevated": "#ffffff",
      "--text-primary": "#1a0c08", "--text-secondary": "#7a5848", "--accent-blue": "#161616", "--accent-green": "#c8a010",
      "--accent-purple": "#974900", "--accent-gold": "#b8a600", "--accent-rose": "#d42e2e",
      "--border-color": "rgba(20, 20, 20, 0.14)", "--error-color": "#c8141a",
    },
  },

  // ── Pair: Roast (Forest Green · Antique Gold) — requested to echo
  // Stumptown Coffee Roasters' packaging, built the same way as the
  // Marshal/Celeste racing-flag pairs above: a real product color measured
  // rather than invented, since (like Ferrari red or Argentine celeste) no
  // single brand hex is published for a coffee bag. Stumptown's site gives
  // no color spec either, so both hero and background were pixel-sampled
  // (dominant-cluster analysis, most-common quantized RGB across a resized
  // grid) directly from product photography of two of their bags, provided
  // in chat on 2026-09-26: the "Hair Bender" bag's dark bag-green
  // (measured ~#3d5a48) for the background, and the "French Roast" bag's
  // arch-logo foil ink — a muted antique gold rather than a bright yellow
  // (measured ~#c9955f) — for the hero, since that duller gold is what
  // actually reads as "Stumptown gold" across their bags, not a shinier
  // one. Background steps use the same ~55%-of-native-lightness shading as
  // the racing-flag family (L 29.6%→16.5% on the primary), then step back
  // up toward the native measured swatch across secondary/tertiary, landing
  // on the native color itself for `--bg-elevated` — so the "elevated"
  // surface is literally the photographed bag color, unscaled.
  // Collision-checked against every hex already in this file (RGB + hue
  // distance): the background green sits >18° of hue from its nearest
  // neighbor (a much bluer teal), clear of the several muted-brown/red
  // heroes already present near this lightness. The gold hero is closer in
  // hue to existing warm-ochre accents (a crowded region in a 400+-theme
  // file) than the green is, but distinctly duller/browner than any of
  // them — reusing the racing family's own shared bottle-yellow hero
  // (#F7D000) was considered and rejected for being an exact, not merely
  // adjacent, duplicate. `accent-purple` reuses a second measured Stumptown
  // color — the "Hundred Mile" bag's toasted orange-red (~#DA6044) — rather
  // than a literal purple, the same "no real purple, borrow a warm
  // complementary" device Blaze and the racing-flag pairs already use.
  // `error-color` stays the file's standard red, same reasoning as those
  // pairs: a semantic signal color, not part of this theme's identity. ──
  {
    id: "dark-roast", name: "Roast", category: "standard", mode: "dark", pairId: "roast",
    preview: { bg: "#223228", fg: "#ffffff", accent: "#c9955f", secondary: "#2b3f32" },
    vars: {
      "--bg-primary": "#223228", "--bg-secondary": "#2b3f32", "--bg-tertiary": "#344c3d", "--bg-elevated": "#3d5a48",
      "--text-primary": "#ffffff", "--text-secondary": "#e3f2e9", "--accent-blue": "#c9955f", "--accent-green": "#7ebe96",
      "--accent-purple": "#da6044", "--accent-gold": "#e0b88f", "--accent-rose": "#e5826b",
      "--border-color": "rgba(201, 149, 95, 0.18)", "--error-color": "#ed1c24",
    },
  },
  {
    id: "light-roast", name: "Roast", category: "standard", mode: "light", pairId: "roast",
    preview: { bg: "#f8f6f0", fg: "#1a1512", accent: "#895d2e", secondary: "#f0ece2" },
    vars: {
      "--bg-primary": "#f8f6f0", "--bg-secondary": "#f0ece2", "--bg-tertiary": "#e6ddc9", "--bg-elevated": "#ffffff",
      "--text-primary": "#1a1512", "--text-secondary": "#6b5d4f", "--accent-blue": "#895d2e", "--accent-green": "#356447",
      "--accent-purple": "#aa3b22", "--accent-gold": "#a56c31", "--accent-rose": "#c44427",
      "--border-color": "rgba(137, 93, 46, 0.14)", "--error-color": "#c8141a",
    },
  },

  // ── Pair: Foundry (Indigo Navy · Antique Gold) — Roast's Stumptown
  // sibling: a second real bag color (the "Wildwood" bag's dark cover,
  // measured ~#3f4459) instead of Roast's green, same shared hero gold
  // (#c9955f / #895d2e light — see Roast's comment for how that hero was
  // sourced and why), same accent-purple/gold/rose/error/light-mode-cream
  // reused verbatim, exactly how Marshal and Celeste share everything but
  // background with Circuit. The measured navy was only 17% saturated —
  // fine for a bag photo, but shaded straight through it read as a generic
  // dark gray, indistinguishable from dozens of existing near-black themes
  // in this file. Saturation was deliberately raised to ~46% (native hue
  // and ~55%-of-native lightness kept, per the family's shading math)
  // so it reads as a distinct indigo rather than blending into that crowd
  // — a documented departure from the literal photograph, not a fabricated
  // color: the hue itself is real, only its intensity is turned up.
  // Collision-checked the same way as Roast: navy/blue is the single most
  // populated hue region in a 400+-theme file, so every shade of it sits
  // within ~10 RGB units of *something* — there is no genuinely empty navy
  // left. This value was picked as the local best (closest neighbors run
  // 5-13 RGB units / 1-10° hue away) rather than a false claim of full
  // clearance, and it sits far from Roast's own green (>100 RGB units) so
  // the two don't read as near-duplicates of each other. `accent-green`
  // borrows the racing family's exact bright yellow (`#ffe44d` dark /
  // `#c8a010` light) rather than a real green, same "no green in this
  // theme's identity" device Marshal/Celeste/Giallo use. ──
  {
    id: "dark-foundry", name: "Foundry", category: "standard", mode: "dark", pairId: "foundry",
    preview: { bg: "#171e3d", fg: "#ffffff", accent: "#c9955f", secondary: "#1d264d" },
    vars: {
      "--bg-primary": "#171e3d", "--bg-secondary": "#1d264d", "--bg-tertiary": "#232e5e", "--bg-elevated": "#29366f",
      "--text-primary": "#ffffff", "--text-secondary": "#dde0ee", "--accent-blue": "#c9955f", "--accent-green": "#ffe44d",
      "--accent-purple": "#da6044", "--accent-gold": "#e0b88f", "--accent-rose": "#e5826b",
      "--border-color": "rgba(201, 149, 95, 0.18)", "--error-color": "#ed1c24",
    },
  },
  {
    id: "light-foundry", name: "Foundry", category: "standard", mode: "light", pairId: "foundry",
    preview: { bg: "#f8f6f0", fg: "#1a1512", accent: "#895d2e", secondary: "#f0ece2" },
    vars: {
      "--bg-primary": "#f8f6f0", "--bg-secondary": "#f0ece2", "--bg-tertiary": "#e6ddc9", "--bg-elevated": "#ffffff",
      "--text-primary": "#1a1512", "--text-secondary": "#6b5d4f", "--accent-blue": "#895d2e", "--accent-green": "#c8a010",
      "--accent-purple": "#aa3b22", "--accent-gold": "#a56c31", "--accent-rose": "#c44427",
      "--border-color": "rgba(137, 93, 46, 0.14)", "--error-color": "#c8141a",
    },
  },

  // ── Pair: Ember (Toasted Orange · Antique Gold) — third Stumptown
  // sibling: the "Hundred Mile" bag's orange-red (measured ~#da6044, native
  // L 56%/S 67% — already reused as this family's shared accent-purple, see
  // Roast's comment), shaded the same ~55%-of-native-lightness way as the
  // rest of the family. Its first shading pass (L 56%→31% on the primary,
  // matching the family's usual ratio) still put the primary/elevated steps
  // close enough to the shared gold hero's own brightness that the hero
  // nearly vanished against the lighter steps (contrast ~1.4-2 at the top of
  // the ramp) — a real usability problem the earlier pairs didn't hit
  // because their native photos were already dark. Shaded harder instead (L
  // 56%→16% on the primary, ~30% steps in a tighter cluster) to keep the
  // whole ramp dark enough for the hero to read clearly at every step
  // (contrast 3.5-5.75). Collision-checked like its siblings: this orange
  // sits ~12° of hue from Marshal's Ferrari red at a similar saturation —
  // the closest of any Stumptown pair to a sibling from a different family,
  // flagged here rather than hidden, but the two still measure >17 RGB
  // units apart. Shares every non-background value with Roast/Foundry —
  // hero, `accent-purple` (the native swatch itself), `accent-gold`,
  // `accent-rose`, `error-color`, and the light-mode template. ──
  {
    id: "dark-ember", name: "Ember", category: "standard", mode: "dark", pairId: "ember",
    preview: { bg: "#44180d", fg: "#ffffff", accent: "#c9955f", secondary: "#581f11" },
    vars: {
      "--bg-primary": "#44180d", "--bg-secondary": "#581f11", "--bg-tertiary": "#6c2615", "--bg-elevated": "#7e2c19",
      "--text-primary": "#ffffff", "--text-secondary": "#eee0dd", "--accent-blue": "#c9955f", "--accent-green": "#ffe44d",
      "--accent-purple": "#da6044", "--accent-gold": "#e0b88f", "--accent-rose": "#e5826b",
      "--border-color": "rgba(201, 149, 95, 0.18)", "--error-color": "#ed1c24",
    },
  },
  {
    id: "light-ember", name: "Ember", category: "standard", mode: "light", pairId: "ember",
    preview: { bg: "#f8f6f0", fg: "#1a1512", accent: "#895d2e", secondary: "#f0ece2" },
    vars: {
      "--bg-primary": "#f8f6f0", "--bg-secondary": "#f0ece2", "--bg-tertiary": "#e6ddc9", "--bg-elevated": "#ffffff",
      "--text-primary": "#1a1512", "--text-secondary": "#6b5d4f", "--accent-blue": "#895d2e", "--accent-green": "#c8a010",
      "--accent-purple": "#aa3b22", "--accent-gold": "#a56c31", "--accent-rose": "#c44427",
      "--border-color": "rgba(137, 93, 46, 0.14)", "--error-color": "#c8141a",
    },
  },

  // ── Pair: Kraft (Roasted Tan · Antique Gold) — fourth Stumptown sibling:
  // the "Trapper Creek" decaf bag's warm tan (measured ~#c48557, native L
  // 56%/S 48%), shaded the same deliberately-harder way as Ember and for the
  // identical reason (native photo too light for the shared hero to read
  // against it at native shading) — L 56%→17% on the primary. This is the
  // most collision-prone Stumptown pair: warm tan/brown sits in one of the
  // most densely populated hue bands in this file (Umber, Cavern Clay,
  // Urbane Bronze and others all live nearby), and its closest neighbors run
  // as near as 5.5 RGB units at effectively the same hue. Kept anyway,
  // unlike Homestead's mustard-gold (rejected — see below) and Holler
  // Mtn's steel-blue (rejected — see below), because it reads as a distinct
  // *warm coffee-tan* next to Roast's green and Foundry's navy rather than
  // duplicating an existing pair's whole identity, only individual nearby
  // hex values. Shares every non-background value with its siblings. ──
  {
    id: "dark-kraft", name: "Kraft", category: "standard", mode: "dark", pairId: "kraft",
    preview: { bg: "#402817", fg: "#ffffff", accent: "#c9955f", secondary: "#52331d" },
    vars: {
      "--bg-primary": "#402817", "--bg-secondary": "#52331d", "--bg-tertiary": "#643e23", "--bg-elevated": "#734828",
      "--text-primary": "#ffffff", "--text-secondary": "#eee4dd", "--accent-blue": "#c9955f", "--accent-green": "#ffe44d",
      "--accent-purple": "#da6044", "--accent-gold": "#e0b88f", "--accent-rose": "#e5826b",
      "--border-color": "rgba(201, 149, 95, 0.18)", "--error-color": "#ed1c24",
    },
  },
  {
    id: "light-kraft", name: "Kraft", category: "standard", mode: "light", pairId: "kraft",
    preview: { bg: "#f8f6f0", fg: "#1a1512", accent: "#895d2e", secondary: "#f0ece2" },
    vars: {
      "--bg-primary": "#f8f6f0", "--bg-secondary": "#f0ece2", "--bg-tertiary": "#e6ddc9", "--bg-elevated": "#ffffff",
      "--text-primary": "#1a1512", "--text-secondary": "#6b5d4f", "--accent-blue": "#895d2e", "--accent-green": "#c8a010",
      "--accent-purple": "#aa3b22", "--accent-gold": "#a56c31", "--accent-rose": "#c44427",
      "--border-color": "rgba(137, 93, 46, 0.14)", "--error-color": "#c8141a",
    },
  },

  // ── Pair: Claim (Homestead Gold · Ledger Navy) — Homestead's bag was
  // rejected earlier in this section's history for reading too close to
  // Giallo's existing golden-olive once shaded, and for having no hero that
  // would show up against its own gold — the shared family gold obviously
  // can't. Revisited using the bag's *own* accent instead of borrowing the
  // family one: its navy label block (measured ~#39426b) as hero. Went
  // through three rounds on request. First kept the background near native
  // brightness with dark text; second darkened it (L 63.7%→30%, hero to L
  // 15% so the two didn't converge) so white text-primary could work like
  // every other Stumptown pair; third darkened the background further still
  // (L 63.7%→16%, matching Ember/Kraft/Hollow's depth) and — this is the
  // part the first two rounds got wrong — brightened the hero rather than
  // darkening it further, because `--accent-blue` isn't only a button fill
  // in this codebase: `.app-title` and `.panel-tab.active` render it as
  // literal small text directly on `--bg-secondary`/other bg tiers
  // (`vibecoder/src/App.css`), which a dark-on-dark hero fails outright —
  // that's what made the wordmark unreadable in review. Every other
  // Stumptown hero already sits well above its own bg-secondary in
  // lightness for exactly this reason (contrast 4.3-5.5); Claim's navy
  // didn't, because navy's blue-weighted luminance formula makes "dark
  // navy" read far darker than a gold or orange hero at a similar HSL
  // lightness would. Fix: keep the hue (229°) but move the hero to L 78%
  // (`#a8b3e6`, a bright periwinkle rather than a literal "navy") — hero
  // vs `--bg-secondary` is 4.59 (clears the same bar the rest of the family
  // hits), vs `--bg-primary` 5.93, softening at the lighter tiers the same
  // way every darkened Stumptown pair's hero does. Button/status-bar text
  // is unaffected either way: `--btn-primary-fg` is auto-computed
  // (`bestFgForBg`) to whichever of black/white contrasts best against
  // the hero, so brightening it just flips that to black (10.26 contrast)
  // instead of breaking anything. Collision-checked like its siblings: the
  // gold background still sits close to Giallo's own (as close as 3.7 RGB
  // units / 0.2° hue at the nearest step) — real, unrelated to this
  // hero/lightness question, flagged for the same review the rest of the
  // family got. Light mode is unchanged — the raw measured navy already had
  // excellent contrast (8.97) on the shared cream and was never the
  // problem. `accent-purple`/`gold`/`rose`/`error` reuse the shared family
  // values; `accent-green` is a fresh pop (`#30a66b` dark / `#206f47`
  // light) since gold-on-gold and navy-on-navy were both already spoken
  // for. ──
  {
    id: "dark-claim", name: "Claim", category: "standard", mode: "dark", pairId: "claim",
    preview: { bg: "#42340f", fg: "#ffffff", accent: "#a8b3e6", secondary: "#564414" },
    vars: {
      "--bg-primary": "#42340f", "--bg-secondary": "#564414", "--bg-tertiary": "#6a5419", "--bg-elevated": "#7c621d",
      "--text-primary": "#ffffff", "--text-secondary": "#ede9de", "--accent-blue": "#a8b3e6", "--accent-green": "#30a66b",
      "--accent-purple": "#da6044", "--accent-gold": "#e0b88f", "--accent-rose": "#e5826b",
      "--border-color": "rgba(168, 179, 230, 0.18)", "--error-color": "#ed1c24",
    },
  },
  {
    id: "light-claim", name: "Claim", category: "standard", mode: "light", pairId: "claim",
    preview: { bg: "#f8f6f0", fg: "#1a1512", accent: "#39426b", secondary: "#f0ece2" },
    vars: {
      "--bg-primary": "#f8f6f0", "--bg-secondary": "#f0ece2", "--bg-tertiary": "#e6ddc9", "--bg-elevated": "#ffffff",
      "--text-primary": "#1a1512", "--text-secondary": "#6b5d4f", "--accent-blue": "#39426b", "--accent-green": "#206f47",
      "--accent-purple": "#aa3b22", "--accent-gold": "#a56c31", "--accent-rose": "#c44427",
      "--border-color": "rgba(57, 66, 107, 0.14)", "--error-color": "#c8141a",
    },
  },

  // ── Pair: Hollow (Fog Steel · Trail Orange) — Holler Mtn's bag was
  // rejected earlier for converging on Roast's/Foundry's own dark values
  // once shaded enough for the shared gold hero to read, or losing all
  // hero contrast if kept at its native pale lightness. Same fix as Claim:
  // use the bag's *own* accent — its bright orange label (measured
  // ~#e68852) — instead of the family gold. Orange-on-blue gave real
  // contrast (5.2 down to 2.9 across the ramp, the same graceful softening
  // at `--bg-elevated` every darkened Stumptown pair has) at the same
  // ~55%-of-native shading the rest of the family uses (L 71.8%→39.5% on
  // the primary), with saturation raised 15%→32% for the same
  // don't-blend-into-generic-dark-gray reason Foundry's was. That shading
  // depth is exactly the territory Roast/Foundry already occupy, so this
  // was checked directly against both rather than just against the file at
  // large: 17.8-19.4 RGB units / 30-57° of hue away from each at the
  // primary step — closer than Roast and Foundry are to each other, but
  // clearly a different color, not a duplicate. Against the file as a
  // whole this hue band is as crowded as navy was for Foundry (nearest
  // nieghbor 3.7 RGB units / 0.7° hue) — noted, not hidden, same as every
  // other close call in this family. `accent-purple`/`gold`/`rose`/`error`
  // reuse the shared family values despite the hero no longer being the
  // shared gold, matching how Ember and Kraft keep those slots even though
  // their own hero situation differs from Roast/Foundry's too. ──
  {
    id: "dark-hollow", name: "Hollow", category: "standard", mode: "dark", pairId: "hollow",
    preview: { bg: "#1d3039", fg: "#ffffff", accent: "#e68852", secondary: "#263e4a" },
    vars: {
      "--bg-primary": "#1d3039", "--bg-secondary": "#263e4a", "--bg-tertiary": "#2e4c5a", "--bg-elevated": "#365868",
      "--text-primary": "#ffffff", "--text-secondary": "#dee8ed", "--accent-blue": "#e68852", "--accent-green": "#ffe44d",
      "--accent-purple": "#da6044", "--accent-gold": "#e0b88f", "--accent-rose": "#e5826b",
      "--border-color": "rgba(230, 136, 82, 0.18)", "--error-color": "#ed1c24",
    },
  },
  {
    id: "light-hollow", name: "Hollow", category: "standard", mode: "light", pairId: "hollow",
    preview: { bg: "#f8f6f0", fg: "#1a1512", accent: "#a14917", secondary: "#f0ece2" },
    vars: {
      "--bg-primary": "#f8f6f0", "--bg-secondary": "#f0ece2", "--bg-tertiary": "#e6ddc9", "--bg-elevated": "#ffffff",
      "--text-primary": "#1a1512", "--text-secondary": "#6b5d4f", "--accent-blue": "#a14917", "--accent-green": "#c8a010",
      "--accent-purple": "#aa3b22", "--accent-gold": "#a56c31", "--accent-rose": "#c44427",
      "--border-color": "rgba(161, 73, 23, 0.14)", "--error-color": "#c8141a",
    },
  },

  // ── Pair: Sagebrush (Olive Sage · Bright Lime) — first Stumptown pair
  // sourced from the *site's* own page backdrop rather than a bag photo:
  // stumptowncoffee.com color-codes each product page's background, and the
  // French Press page's dusty sage (measured ~#72826c, native H 104°/S
  // 9%/L 47%) is a genuinely different hue from every bag-derived Stumptown
  // pair so far — 38-40° of hue from Roast's forest green, not a repeat of
  // it. Native saturation was almost gray (9%), so it was boosted to 35%
  // for the same don't-blend-into-generic-dark-gray reason Foundry's was.
  // Shading used Giallo's proven lightness band (26.3%→35.5% across the
  // four steps) instead of this family's usual ~55%-of-native cut: the
  // wider spread used for Ember/Kraft/Hollow/Claim happened to work there,
  // but Claim's review round showed that band can leave a hero without
  // enough separation from `--bg-secondary` to work as literal text (see
  // Claim's comment) — using an already-proven band sidesteps re-solving
  // that per pair. The page's own on-screen buttons are near-black, but
  // black measures poorly as a *hero* here for the identical luminance
  // reason Claim's original navy failed: green's high luminance weight
  // (0.7152 of the relative-luminance formula) means black-on-green needs
  // a much lighter green than black-on-yellow (which is why Giallo's own
  // black hero works fine at this exact lightness band) to hit real
  // contrast. Used a bright lime instead, same hue, brightened to L 75%
  // (`#afe29c`) — hero-vs-`--bg-secondary` is 4.48, matching the bar every
  // other pair in this family clears, `--btn-primary-fg` auto-flips to
  // black against it. `accent-purple`/`gold`/`rose`/`error` reuse the
  // shared family values; `accent-green` holds the shared family gold
  // since the hero already covers this pair's green identity. ──
  {
    id: "dark-sagebrush", name: "Sagebrush", category: "standard", mode: "dark", pairId: "sagebrush",
    preview: { bg: "#385b2c", fg: "#ffffff", accent: "#afe29c", secondary: "#406631" },
    vars: {
      "--bg-primary": "#385b2c", "--bg-secondary": "#406631", "--bg-tertiary": "#467036", "--bg-elevated": "#4c7a3b",
      "--text-primary": "#ffffff", "--text-secondary": "#e3ecdf", "--accent-blue": "#afe29c", "--accent-green": "#c9955f",
      "--accent-purple": "#da6044", "--accent-gold": "#e0b88f", "--accent-rose": "#e5826b",
      "--border-color": "rgba(175, 226, 156, 0.18)", "--error-color": "#ed1c24",
    },
  },
  {
    id: "light-sagebrush", name: "Sagebrush", category: "standard", mode: "light", pairId: "sagebrush",
    preview: { bg: "#f8f6f0", fg: "#1a1512", accent: "#3d7e25", secondary: "#f0ece2" },
    vars: {
      "--bg-primary": "#f8f6f0", "--bg-secondary": "#f0ece2", "--bg-tertiary": "#e6ddc9", "--bg-elevated": "#ffffff",
      "--text-primary": "#1a1512", "--text-secondary": "#6b5d4f", "--accent-blue": "#3d7e25", "--accent-green": "#a56c31",
      "--accent-purple": "#aa3b22", "--accent-gold": "#a56c31", "--accent-rose": "#c44427",
      "--border-color": "rgba(61, 126, 37, 0.14)", "--error-color": "#c8141a",
    },
  },

  // ── Pair: Adobe (Dusty Terracotta · Antique Gold) — second Stumptown pair
  // from the site's own page backdrop: the Hundred Mile page's dusty salmon
  // (measured ~#d68079, native H 4.5°/S 53%/L 66%) — only ~7° of hue from
  // Ember's Hundred-Mile-*bag* orange, but visibly a different color at
  // native saturation and lightness (dusty/muted vs Ember's vivid toasted
  // orange), so both were kept rather than treating one as redundant.
  // Shaded on the same proven Giallo band as Sagebrush, for the same
  // reason. The page's own black button measures even worse here than for
  // Sagebrush — red has the *lowest* weight (0.2126) in the relative-
  // luminance formula of any primary hue, so a black hero against a red
  // background needs a much lighter red than this band provides before
  // black gets real contrast. Used the shared family gold hero instead
  // (contrast 3.1-4.3 across the ramp, in line with the rest of the
  // family) rather than chase a hero bright enough to rescue black, the
  // same trade favored for Ember/Kraft's own hero choice. `accent-purple`
  // and `accent-rose` deliberately do *not* reuse the shared Hundred-Mile-
  // orange/coral values here — both sit within ~7° of this pair's own
  // background hue and would nearly vanish into it — using Roast's own
  // green pop (`#7ebe96` dark / `#356447` light) instead, the one
  // Stumptown accent color that reads as a clear complementary against a
  // warm red background. `accent-gold`/`error` reuse the shared values. ──
  {
    id: "dark-adobe", name: "Adobe", category: "standard", mode: "dark", pairId: "adobe",
    preview: { bg: "#68241e", fg: "#ffffff", accent: "#c9955f", secondary: "#752822" },
    vars: {
      "--bg-primary": "#68241e", "--bg-secondary": "#752822", "--bg-tertiary": "#802c25", "--bg-elevated": "#8c3029",
      "--text-primary": "#ffffff", "--text-secondary": "#eddfde", "--accent-blue": "#c9955f", "--accent-green": "#ffe44d",
      "--accent-purple": "#7ebe96", "--accent-gold": "#e0b88f", "--accent-rose": "#7ebe96",
      "--border-color": "rgba(201, 149, 95, 0.18)", "--error-color": "#ed1c24",
    },
  },
  {
    id: "light-adobe", name: "Adobe", category: "standard", mode: "light", pairId: "adobe",
    preview: { bg: "#f8f6f0", fg: "#1a1512", accent: "#895d2e", secondary: "#f0ece2" },
    vars: {
      "--bg-primary": "#f8f6f0", "--bg-secondary": "#f0ece2", "--bg-tertiary": "#e6ddc9", "--bg-elevated": "#ffffff",
      "--text-primary": "#1a1512", "--text-secondary": "#6b5d4f", "--accent-blue": "#895d2e", "--accent-green": "#c8a010",
      "--accent-purple": "#356447", "--accent-gold": "#a56c31", "--accent-rose": "#356447",
      "--border-color": "rgba(137, 93, 46, 0.14)", "--error-color": "#c8141a",
    },
  },

  // ── 2026 Color-of-the-Year inspired pairs ──
  // Palette direction drawn from the 2026 paint-industry color-of-the-year
  // roundup (younghouselove.com/2026-color-of-the-year) — burnt umbers,
  // warm khakis, smokey jades, muted mahoganies and eucalyptus greens, deep
  // plums and warm ochres — plus two callbacks to recent Pantone COTY hues
  // (peach, mocha) for continuity. Named for the hue itself, not any paint
  // brand's marketing name — see the "Remove brand-named themes" commit.

  // ── Pair: Umber (Burnt Umber / Warm Clay) ──
  {
    id: "dark-umber", name: "Umber", category: "standard", mode: "dark", pairId: "umber",
    preview: { bg: "#1c1613", fg: "#ece3da", accent: "#c17a54", secondary: "#241c18" },
    vars: {
      "--bg-primary": "#1c1613", "--bg-secondary": "#241c18", "--bg-tertiary": "#2d231e", "--bg-elevated": "#362a23",
      "--text-primary": "#ece3da", "--text-secondary": "#a08f80", "--accent-blue": "#c17a54", "--accent-green": "#8a9a6b",
      "--accent-purple": "#a67c8e", "--accent-gold": "#d1a35c", "--accent-rose": "#c1614f",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#e0574a",
    },
  },
  {
    id: "light-umber", name: "Umber", category: "standard", mode: "light", pairId: "umber",
    preview: { bg: "#faf5f0", fg: "#2b211b", accent: "#a8623d", secondary: "#f0e6dc" },
    vars: {
      "--bg-primary": "#faf5f0", "--bg-secondary": "#f0e6dc", "--bg-tertiary": "#e6d7c8", "--bg-elevated": "#ffffff",
      "--text-primary": "#2b211b", "--text-secondary": "#7d6a5c", "--accent-blue": "#a8623d", "--accent-green": "#6f7d52",
      "--accent-purple": "#8f6070", "--accent-gold": "#b3822f", "--accent-rose": "#a8493a",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#c2402f",
    },
  },
  // ── Pair: Khaki (Khaki Dusk / Khaki Field) ── dark-khaki's bg lifted from
  // near-black (#17150f, L≈8%) to Cobalt/Monokai's mid-dark range (#33301f,
  // L≈17%, between Monokai's #272822 at ≈15% and Cobalt's #193549 at ≈20%)
  // per explicit request — the family stays khaki/olive, just not as dark.
  {
    id: "dark-khaki", name: "Khaki", category: "standard", mode: "dark", pairId: "khaki",
    preview: { bg: "#33301f", fg: "#f1ecd8", accent: "#d4b96a", secondary: "#423e29" },
    vars: {
      "--bg-primary": "#33301f", "--bg-secondary": "#423e29", "--bg-tertiary": "#504a31", "--bg-elevated": "#5e563a",
      "--text-primary": "#f1ecd8", "--text-secondary": "#b3a888", "--accent-blue": "#d4b96a", "--accent-green": "#9bbf5e",
      "--accent-purple": "#b79bd1", "--accent-gold": "#e8c15a", "--accent-rose": "#d98a6c",
      "--border-color": "rgba(255, 255, 255, 0.08)", "--error-color": "#e0574a",
    },
  },
  {
    id: "light-khaki", name: "Khaki", category: "standard", mode: "light", pairId: "khaki",
    preview: { bg: "#faf8f2", fg: "#2a2718", accent: "#8a7346", secondary: "#f1ecdf" },
    vars: {
      "--bg-primary": "#faf8f2", "--bg-secondary": "#f1ecdf", "--bg-tertiary": "#e6ddc8", "--bg-elevated": "#ffffff",
      "--text-primary": "#2a2718", "--text-secondary": "#756d54", "--accent-blue": "#8a7346", "--accent-green": "#5f7548",
      "--accent-purple": "#776488", "--accent-gold": "#a3812e", "--accent-rose": "#9c6650",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#b8503f",
    },
  },
  // ── Pair: Jade (Jade Smoke / Jade Mist) ──
  {
    id: "dark-jade", name: "Jade", category: "standard", mode: "dark", pairId: "jade",
    preview: { bg: "#0e1613", fg: "#dbe8e0", accent: "#4f9c86", secondary: "#16211c" },
    vars: {
      "--bg-primary": "#0e1613", "--bg-secondary": "#16211c", "--bg-tertiary": "#1d2b24", "--bg-elevated": "#24352c",
      "--text-primary": "#dbe8e0", "--text-secondary": "#7fa090", "--accent-blue": "#4f9c86", "--accent-green": "#5cae7f",
      "--accent-purple": "#7c9ba0", "--accent-gold": "#c9a24d", "--accent-rose": "#c17d8f",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#e0645a",
    },
  },
  {
    id: "light-jade", name: "Jade", category: "standard", mode: "light", pairId: "jade",
    preview: { bg: "#f2f8f5", fg: "#16241d", accent: "#2c7d68", secondary: "#e2efe8" },
    vars: {
      "--bg-primary": "#f2f8f5", "--bg-secondary": "#e2efe8", "--bg-tertiary": "#cfe3d8", "--bg-elevated": "#ffffff",
      "--text-primary": "#16241d", "--text-secondary": "#547567", "--accent-blue": "#2c7d68", "--accent-green": "#3d8a5e",
      "--accent-purple": "#5c7d82", "--accent-gold": "#a3812c", "--accent-rose": "#a3596a",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#c2483d",
    },
  },
  // ── Pair: Mahogany (Mahogany Ember / Mahogany Bloom) ──
  {
    id: "dark-mahogany", name: "Mahogany", category: "standard", mode: "dark", pairId: "mahogany",
    preview: { bg: "#180d0b", fg: "#f0ddd5", accent: "#b1543c", secondary: "#241512" },
    vars: {
      "--bg-primary": "#180d0b", "--bg-secondary": "#241512", "--bg-tertiary": "#301c17", "--bg-elevated": "#3c241d",
      "--text-primary": "#f0ddd5", "--text-secondary": "#b08b7f", "--accent-blue": "#b1543c", "--accent-green": "#8a9560",
      "--accent-purple": "#a06a72", "--accent-gold": "#cf9a4e", "--accent-rose": "#c65847",
      "--border-color": "rgba(255, 255, 255, 0.07)", "--error-color": "#e2543f",
    },
  },
  {
    id: "light-mahogany", name: "Mahogany", category: "standard", mode: "light", pairId: "mahogany",
    preview: { bg: "#fbf3f0", fg: "#2c1712", accent: "#953d29", secondary: "#f3e2dc" },
    vars: {
      "--bg-primary": "#fbf3f0", "--bg-secondary": "#f3e2dc", "--bg-tertiary": "#e8cec4", "--bg-elevated": "#ffffff",
      "--text-primary": "#2c1712", "--text-secondary": "#886059", "--accent-blue": "#953d29", "--accent-green": "#6c7746",
      "--accent-purple": "#86505a", "--accent-gold": "#a97a28", "--accent-rose": "#a8402f",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#b83326",
    },
  },
  // ── Pair: Eucalyptus (Eucalyptus Grove / Eucalyptus Air) ──
  {
    id: "dark-eucalyptus", name: "Eucalyptus", category: "standard", mode: "dark", pairId: "eucalyptus",
    preview: { bg: "#111614", fg: "#dde6e0", accent: "#6fa393", secondary: "#19221f" },
    vars: {
      "--bg-primary": "#111614", "--bg-secondary": "#19221f", "--bg-tertiary": "#212c27", "--bg-elevated": "#2a3630",
      "--text-primary": "#dde6e0", "--text-secondary": "#8ba299", "--accent-blue": "#6fa393", "--accent-green": "#7cae82",
      "--accent-purple": "#8c9bab", "--accent-gold": "#c3a561", "--accent-rose": "#b98a8f",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#d97466",
    },
  },
  {
    id: "light-eucalyptus", name: "Eucalyptus", category: "standard", mode: "light", pairId: "eucalyptus",
    preview: { bg: "#f4f8f6", fg: "#17211d", accent: "#3f7d6c", secondary: "#e6efeb" },
    vars: {
      "--bg-primary": "#f4f8f6", "--bg-secondary": "#e6efeb", "--bg-tertiary": "#d4e2dc", "--bg-elevated": "#ffffff",
      "--text-primary": "#17211d", "--text-secondary": "#5f7a71", "--accent-blue": "#3f7d6c", "--accent-green": "#4c8452",
      "--accent-purple": "#5f7188", "--accent-gold": "#9c7f2e", "--accent-rose": "#a26065",
      "--border-color": "rgba(0, 0, 0, 0.07)", "--error-color": "#bb4c3d",
    },
  },
  // ── Pair: Lagoon (Lagoon Depth / Lagoon Shallows) ──
  {
    id: "dark-lagoon", name: "Lagoon", category: "standard", mode: "dark", pairId: "lagoon",
    preview: { bg: "#071618", fg: "#dcf1f0", accent: "#2fa9a1", secondary: "#0e2226" },
    vars: {
      "--bg-primary": "#071618", "--bg-secondary": "#0e2226", "--bg-tertiary": "#142e32", "--bg-elevated": "#1c3a3f",
      "--text-primary": "#dcf1f0", "--text-secondary": "#75a6a4", "--accent-blue": "#2fa9a1", "--accent-green": "#52b788",
      "--accent-purple": "#7c93c4", "--accent-gold": "#d9b64f", "--accent-rose": "#d17789",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#e26557",
    },
  },
  {
    id: "light-lagoon", name: "Lagoon", category: "standard", mode: "light", pairId: "lagoon",
    preview: { bg: "#eefbfa", fg: "#072220", accent: "#12897f", secondary: "#dcf1ef" },
    vars: {
      "--bg-primary": "#eefbfa", "--bg-secondary": "#dcf1ef", "--bg-tertiary": "#c3e6e2", "--bg-elevated": "#ffffff",
      "--text-primary": "#072220", "--text-secondary": "#4c7d79", "--accent-blue": "#12897f", "--accent-green": "#2c9160",
      "--accent-purple": "#4f6ba0", "--accent-gold": "#a3821f", "--accent-rose": "#b8536a",
      "--border-color": "rgba(0, 0, 0, 0.07)", "--error-color": "#c14636",
    },
  },
  // ── Pair: Plum (Plum Velvet / Plum Blossom) ──
  {
    id: "dark-plum", name: "Plum", category: "standard", mode: "dark", pairId: "plum",
    preview: { bg: "#140a15", fg: "#f0dcee", accent: "#9b4f9e", secondary: "#1f1121" },
    vars: {
      "--bg-primary": "#140a15", "--bg-secondary": "#1f1121", "--bg-tertiary": "#2a172c", "--bg-elevated": "#351f38",
      "--text-primary": "#f0dcee", "--text-secondary": "#a888a6", "--accent-blue": "#9b4f9e", "--accent-green": "#7fa06f",
      "--accent-purple": "#b565b8", "--accent-gold": "#d1a052", "--accent-rose": "#cd5f8a",
      "--border-color": "rgba(255, 255, 255, 0.07)", "--error-color": "#e0566a",
    },
  },
  {
    id: "light-plum", name: "Plum", category: "standard", mode: "light", pairId: "plum",
    preview: { bg: "#faf3fa", fg: "#260d27", accent: "#7a3980", secondary: "#f1e1f0" },
    vars: {
      "--bg-primary": "#faf3fa", "--bg-secondary": "#f1e1f0", "--bg-tertiary": "#e4c9e2", "--bg-elevated": "#ffffff",
      "--text-primary": "#260d27", "--text-secondary": "#82628a", "--accent-blue": "#7a3980", "--accent-green": "#5c7d4c",
      "--accent-purple": "#8f4796", "--accent-gold": "#a67c2c", "--accent-rose": "#a8446a",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#b83a4c",
    },
  },
  // ── Pair: Ochre (Ochre Clay / Ochre Sand) ──
  {
    id: "dark-ochre", name: "Ochre", category: "standard", mode: "dark", pairId: "ochre",
    preview: { bg: "#171209", fg: "#f0e4cc", accent: "#c99a3f", secondary: "#241c0f" },
    vars: {
      "--bg-primary": "#171209", "--bg-secondary": "#241c0f", "--bg-tertiary": "#302514", "--bg-elevated": "#3c2f1a",
      "--text-primary": "#f0e4cc", "--text-secondary": "#b39d6f", "--accent-blue": "#c99a3f", "--accent-green": "#93a15a",
      "--accent-purple": "#a2879e", "--accent-gold": "#d9ad4a", "--accent-rose": "#c17a5a",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#d96751",
    },
  },
  {
    id: "light-ochre", name: "Ochre", category: "standard", mode: "light", pairId: "ochre",
    preview: { bg: "#fbf7ee", fg: "#2c2410", accent: "#9c7422", secondary: "#f3e9d2" },
    vars: {
      "--bg-primary": "#fbf7ee", "--bg-secondary": "#f3e9d2", "--bg-tertiary": "#e8d8ac", "--bg-elevated": "#ffffff",
      "--text-primary": "#2c2410", "--text-secondary": "#7f6f45", "--accent-blue": "#9c7422", "--accent-green": "#6f7d3f",
      "--accent-purple": "#7d647a", "--accent-gold": "#ab8420", "--accent-rose": "#a4593d",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#b8482f",
    },
  },

  // ── Other inspirations: recent Pantone Color-of-the-Year callbacks ──
  // ── Pair: Mocha (Mocha Roast / Mocha Cream) ──
  {
    id: "dark-mocha", name: "Mocha", category: "standard", mode: "dark", pairId: "mocha",
    preview: { bg: "#150f0c", fg: "#ede1d5", accent: "#a97c56", secondary: "#201812" },
    vars: {
      "--bg-primary": "#150f0c", "--bg-secondary": "#201812", "--bg-tertiary": "#2a2018", "--bg-elevated": "#34281e",
      "--text-primary": "#ede1d5", "--text-secondary": "#a6907c", "--accent-blue": "#a97c56", "--accent-green": "#8c9968",
      "--accent-purple": "#9c8299", "--accent-gold": "#c99e5c", "--accent-rose": "#bb7d69",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#d76d55",
    },
  },
  {
    id: "light-mocha", name: "Mocha", category: "standard", mode: "light", pairId: "mocha",
    preview: { bg: "#faf6f1", fg: "#2a2016", accent: "#8a5f3c", secondary: "#f0e6da" },
    vars: {
      "--bg-primary": "#faf6f1", "--bg-secondary": "#f0e6da", "--bg-tertiary": "#e2d2bd", "--bg-elevated": "#ffffff",
      "--text-primary": "#2a2016", "--text-secondary": "#7d6a58", "--accent-blue": "#8a5f3c", "--accent-green": "#6c7a49",
      "--accent-purple": "#7a6178", "--accent-gold": "#a3792c", "--accent-rose": "#a05f4c",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#b8503a",
    },
  },
  // ── Pair: Peach (Peach Ember / Peach Bellini) ──
  {
    id: "dark-peach", name: "Peach", category: "standard", mode: "dark", pairId: "peach",
    preview: { bg: "#190f0d", fg: "#f5e1d6", accent: "#f0977a", secondary: "#261613" },
    vars: {
      "--bg-primary": "#190f0d", "--bg-secondary": "#261613", "--bg-tertiary": "#331e18", "--bg-elevated": "#40271f",
      "--text-primary": "#f5e1d6", "--text-secondary": "#c0958a", "--accent-blue": "#f0977a", "--accent-green": "#93a86f",
      "--accent-purple": "#c88fa0", "--accent-gold": "#e8ae5c", "--accent-rose": "#ee8a92",
      "--border-color": "rgba(255, 255, 255, 0.07)", "--error-color": "#ee6154",
    },
  },
  {
    id: "light-peach", name: "Peach", category: "standard", mode: "light", pairId: "peach",
    preview: { bg: "#fff5f0", fg: "#2f150e", accent: "#d9713f", secondary: "#fbe6da" },
    vars: {
      "--bg-primary": "#fff5f0", "--bg-secondary": "#fbe6da", "--bg-tertiary": "#f3d1bc", "--bg-elevated": "#ffffff",
      "--text-primary": "#2f150e", "--text-secondary": "#93655a", "--accent-blue": "#d9713f", "--accent-green": "#728041",
      "--accent-purple": "#a8607a", "--accent-gold": "#ba7f26", "--accent-rose": "#c3554e",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#c2402f",
    },
  },
  // ── Pair: Pink (Pink Neon / Pink Petal) ──
  // A true magenta-leaning pink, distinct from Rose's crimson-red — the
  // closest thing here to a soft phone-finish blush pink.
  {
    id: "dark-pink", name: "Pink", category: "standard", mode: "dark", pairId: "pink",
    preview: { bg: "#170e14", fg: "#f5dceb", accent: "#e8639f", secondary: "#241522" },
    vars: {
      "--bg-primary": "#170e14", "--bg-secondary": "#241522", "--bg-tertiary": "#301c2c", "--bg-elevated": "#3c2438",
      "--text-primary": "#f5dceb", "--text-secondary": "#b8829f", "--accent-blue": "#e8639f", "--accent-green": "#6fbf8a",
      "--accent-purple": "#c07cd4", "--accent-gold": "#dba354", "--accent-rose": "#ef5c8a",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#e2495f",
    },
  },
  {
    id: "light-pink", name: "Pink", category: "standard", mode: "light", pairId: "pink",
    preview: { bg: "#fef6fa", fg: "#2e1521", accent: "#c93d80", secondary: "#fbe6f0" },
    vars: {
      "--bg-primary": "#fef6fa", "--bg-secondary": "#fbe6f0", "--bg-tertiary": "#f5cfe0", "--bg-elevated": "#ffffff",
      "--text-primary": "#2e1521", "--text-secondary": "#8a5570", "--accent-blue": "#c93d80", "--accent-green": "#4c8f63",
      "--accent-purple": "#9c4fae", "--accent-gold": "#b3822f", "--accent-rose": "#d1436a",
      "--border-color": "rgba(0, 0, 0, 0.07)", "--error-color": "#c2304a",
    },
  },
  // ── Restored pairs: Tamar, Keswick, Aintree, Giola, Faroe, Patagonia ──
  // Same six palettes the user asked to bring back, carried over unchanged
  // (colors + names) from before today's brand-name purge — but re-added
  // under category "standard" with no Land Rover attribution or "lr-"
  // prefixed ids, per that same purge's naming rule (color/mood name, not
  // a brand's marketing name).

  // ── Pair: Tamar ──
  {
    id: "dark-tamar", name: "Tamar", category: "standard", mode: "dark", pairId: "tamar",
    preview: { bg: "#0d1012", fg: "#e3e6e8", accent: "#6b9cbd", secondary: "#151a1e" },
    vars: {
      "--bg-primary": "#0d1012", "--bg-secondary": "#151a1e", "--bg-tertiary": "#1d252a", "--bg-elevated": "#283239",
      "--text-primary": "#e3e6e8", "--text-secondary": "#768794", "--accent-blue": "#6b9cbd", "--accent-green": "#b96eb4",
      "--accent-purple": "#82b96e", "--accent-gold": "#7972ca", "--accent-rose": "#69bfa8",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f14c4c",
    },
  },
  {
    id: "light-tamar", name: "Tamar", category: "standard", mode: "light", pairId: "tamar",
    preview: { bg: "#f6f8f8", fg: "#1a1f23", accent: "#3f6e8d", secondary: "#ebeeef" },
    vars: {
      "--bg-primary": "#f6f8f8", "--bg-secondary": "#ebeeef", "--bg-tertiary": "#dadfe2", "--bg-elevated": "#ffffff",
      "--text-primary": "#1a1f23", "--text-secondary": "#586974", "--accent-blue": "#3f6e8d", "--accent-green": "#833f7e",
      "--accent-purple": "#51833f", "--accent-gold": "#423b9b", "--accent-rose": "#3a8873",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#dc2626",
    },
  },
  // ── Pair: Keswick ──
  {
    id: "dark-keswick", name: "Keswick", category: "standard", mode: "dark", pairId: "keswick",
    preview: { bg: "#0f110e", fg: "#e6e7e4", accent: "#94bd6b", secondary: "#1a1c17" },
    vars: {
      "--bg-primary": "#0f110e", "--bg-secondary": "#1a1c17", "--bg-tertiary": "#242720", "--bg-elevated": "#30352c",
      "--text-primary": "#e6e7e4", "--text-secondary": "#858d7c", "--accent-blue": "#94bd6b", "--accent-green": "#6eadb9",
      "--accent-purple": "#b96e7b", "--accent-gold": "#72ca81", "--accent-rose": "#bfb069",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f14c4c",
    },
  },
  {
    id: "light-keswick", name: "Keswick", category: "standard", mode: "light", pairId: "keswick",
    preview: { bg: "#f7f8f7", fg: "#1f211c", accent: "#668d3f", secondary: "#edeeec" },
    vars: {
      "--bg-primary": "#f7f8f7", "--bg-secondary": "#edeeec", "--bg-tertiary": "#dee0dc", "--bg-elevated": "#ffffff",
      "--text-primary": "#1f211c", "--text-secondary": "#666e5e", "--accent-blue": "#668d3f", "--accent-green": "#3f7883",
      "--accent-purple": "#833f4a", "--accent-gold": "#3b9b4b", "--accent-rose": "#887b3a",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#dc2626",
    },
  },
  // ── Pair: Aintree ──
  {
    id: "dark-aintree", name: "Aintree", category: "standard", mode: "dark", pairId: "aintree",
    preview: { bg: "#0f3d2e", fg: "#e2e9e7", accent: "#53d5aa", secondary: "#253c34" },
    vars: {
      "--bg-primary": "#0f3d2e", "--bg-secondary": "#253c34", "--bg-tertiary": "#2e473f", "--bg-elevated": "#37574d",
      "--text-primary": "#e2e9e7", "--text-secondary": "#6c9d8d", "--accent-blue": "#53d5aa", "--accent-green": "#7f53d5",
      "--accent-purple": "#d4d553", "--accent-gold": "#5ab4e2", "--accent-rose": "#4eda4f",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f14c4c",
    },
  },
  {
    id: "light-aintree", name: "Aintree", category: "standard", mode: "light", pairId: "aintree",
    preview: { bg: "#f6f9f8", fg: "#192421", accent: "#28a47b", secondary: "#eaf0ee" },
    vars: {
      "--bg-primary": "#f6f9f8", "--bg-secondary": "#eaf0ee", "--bg-tertiary": "#d9e3e0", "--bg-elevated": "#ffffff",
      "--text-primary": "#192421", "--text-secondary": "#507c6e", "--accent-blue": "#28a47b", "--accent-green": "#4e269c",
      "--accent-purple": "#9b9c26", "--accent-gold": "#2083b7", "--accent-rose": "#21a022",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#dc2626",
    },
  },
  // ── Pair: Giola ──
  {
    id: "dark-giola", name: "Giola", category: "standard", mode: "dark", pairId: "giola",
    preview: { bg: "#0c1311", fg: "#e2e9e7", accent: "#58d0b5", secondary: "#141f1d" },
    vars: {
      "--bg-primary": "#0c1311", "--bg-secondary": "#141f1d", "--bg-tertiary": "#1c2c28", "--bg-elevated": "#263b36",
      "--text-primary": "#e2e9e7", "--text-secondary": "#6d9d92", "--accent-blue": "#58d0b5", "--accent-green": "#8d58d0",
      "--accent-purple": "#c3d058", "--accent-gold": "#5ea6de", "--accent-rose": "#53d560",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f14c4c",
    },
  },
  {
    id: "light-giola", name: "Giola", category: "standard", mode: "light", pairId: "giola",
    preview: { bg: "#f6f9f8", fg: "#192422", accent: "#2d9f85", secondary: "#eaf0ef" },
    vars: {
      "--bg-primary": "#f6f9f8", "--bg-secondary": "#eaf0ef", "--bg-tertiary": "#d9e3e1", "--bg-elevated": "#ffffff",
      "--text-primary": "#192422", "--text-secondary": "#507c72", "--accent-blue": "#2d9f85", "--accent-green": "#5a2b97",
      "--accent-purple": "#8c972b", "--accent-gold": "#2474b2", "--accent-rose": "#269c32",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#dc2626",
    },
  },
  // ── Pair: Faroe ──
  {
    id: "dark-faroe", name: "Faroe", category: "standard", mode: "dark", pairId: "faroe",
    preview: { bg: "#0f100e", fg: "#e5e7e4", accent: "#65a249", secondary: "#191b18" },
    vars: {
      "--bg-primary": "#0f100e", "--bg-secondary": "#191b18", "--bg-tertiary": "#232621", "--bg-elevated": "#2f342d",
      "--text-primary": "#e5e7e4", "--text-secondary": "#828b7e", "--accent-blue": "#65a249", "--accent-green": "#6e9fb9",
      "--accent-purple": "#b96f6e", "--accent-gold": "#72ca91", "--accent-rose": "#bebf69",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f14c4c",
    },
  },
  {
    id: "light-faroe", name: "Faroe", category: "standard", mode: "light", pairId: "faroe",
    preview: { bg: "#f7f8f7", fg: "#1e201d", accent: "#52823b", secondary: "#edeeec" },
    vars: {
      "--bg-primary": "#f7f8f7", "--bg-secondary": "#edeeec", "--bg-tertiary": "#dddfdc", "--bg-elevated": "#ffffff",
      "--text-primary": "#1e201d", "--text-secondary": "#646c60", "--accent-blue": "#52823b", "--accent-green": "#3f6b83",
      "--accent-purple": "#83403f", "--accent-gold": "#3b9b5c", "--accent-rose": "#87883a",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#dc2626",
    },
  },
  // ── Pair: Patagonia ──
  {
    id: "dark-patagonia", name: "Patagonia", category: "standard", mode: "dark", pairId: "patagonia",
    preview: { bg: "#10100e", fg: "#e7e6e4", accent: "#bdab6b", secondary: "#1b1b18" },
    vars: {
      "--bg-primary": "#10100e", "--bg-secondary": "#1b1b18", "--bg-tertiary": "#262521", "--bg-elevated": "#34332d",
      "--text-primary": "#e7e6e4", "--text-secondary": "#8b887e", "--accent-blue": "#bdab6b", "--accent-green": "#6eb990",
      "--accent-purple": "#b96eb1", "--accent-gold": "#a3ca72", "--accent-rose": "#bf7369",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f14c4c",
    },
  },
  {
    id: "light-patagonia", name: "Patagonia", category: "standard", mode: "light", pairId: "patagonia",
    preview: { bg: "#e6e4dd", fg: "#20201d", accent: "#8d7c3f", secondary: "#dad9d4" },
    vars: {
      "--bg-primary": "#e6e4dd", "--bg-secondary": "#dad9d4", "--bg-tertiary": "#cbcac5", "--bg-elevated": "#ffffff",
      "--text-primary": "#20201d", "--text-secondary": "#6c6960", "--accent-blue": "#8d7c3f", "--accent-green": "#3f835d",
      "--accent-purple": "#833f7b", "--accent-gold": "#709b3b", "--accent-rose": "#88433a",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#dc2626",
    },
  },
  // ── Restored pairs: Fuji, Santorini, Bronze Umber, Eiger, Indus, Carpathian,
  // Narvik, Silicon, Yulong, Portofino, Gondwana, Pangea, Tasman,
  // Borasco, Woolstone, Deep Sandglow, Sedona, Charente, Petra,
  // Namib, Varesine, Lantau, Velocity, Racing, Pearl, Marine, Balmoral ──
  // Same palettes and names as before today's brand-name purge, re-added
  // under category "standard" with plain ids — same treatment as the
  // Tamar/Keswick/Aintree/Giola/Faroe/Patagonia batch.

  // ── Pair: Fuji ──
  {
    id: "dark-fuji", name: "Fuji", category: "standard", mode: "dark", pairId: "fuji",
    preview: { bg: "#0e0f11", fg: "#e4e6e7", accent: "#6b94bd", secondary: "#171a1c" },
    vars: {
      "--bg-primary": "#0e0f11", "--bg-secondary": "#171a1c", "--bg-tertiary": "#212427", "--bg-elevated": "#2c3035",
      "--text-primary": "#e4e6e7", "--text-secondary": "#7d858c", "--accent-blue": "#6b94bd", "--accent-green": "#b96ead",
      "--accent-purple": "#7bb96e", "--accent-gold": "#8172ca", "--accent-rose": "#69bfb0",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f14c4c",
    },
  },
  {
    id: "light-fuji", name: "Fuji", category: "standard", mode: "light", pairId: "fuji",
    preview: { bg: "#ebeef1", fg: "#1c1f21", accent: "#3f668d", secondary: "#e2e4e6" },
    vars: {
      "--bg-primary": "#ebeef1", "--bg-secondary": "#e2e4e6", "--bg-tertiary": "#d2d5d7", "--bg-elevated": "#ffffff",
      "--text-primary": "#1c1f21", "--text-secondary": "#5f666d", "--accent-blue": "#3f668d", "--accent-green": "#833f78",
      "--accent-purple": "#4a833f", "--accent-gold": "#4b3b9b", "--accent-rose": "#3a887b",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#dc2626",
    },
  },
  // ── Pair: Santorini ──
  // No verified public hex exists for this swatch's real-world name; this is
  // a careful near-black estimate, nudged to stay visually distinct from
  // this set's other near-neutral blacks (Narvik, Carpathian) below.
  {
    id: "dark-santorini", name: "Santorini", category: "standard", mode: "dark", pairId: "santorini",
    preview: { bg: "#100e0d", fg: "#e6e5e5", accent: "#bd866b", secondary: "#1a1817" },
    vars: {
      "--bg-primary": "#100e0d", "--bg-secondary": "#1a1817", "--bg-tertiary": "#252221", "--bg-elevated": "#322f2d",
      "--text-primary": "#e6e5e5", "--text-secondary": "#8b837e", "--accent-blue": "#bd866b", "--accent-green": "#6eb96e",
      "--accent-purple": "#a06eb9", "--accent-gold": "#caca72", "--accent-rose": "#bf6986",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f14c4c",
    },
  },
  {
    id: "light-santorini", name: "Santorini", category: "standard", mode: "light", pairId: "santorini",
    preview: { bg: "#f8f7f7", fg: "#201e1d", accent: "#8d593f", secondary: "#eeeded" },
    vars: {
      "--bg-primary": "#f8f7f7", "--bg-secondary": "#eeeded", "--bg-tertiary": "#dfdddd", "--bg-elevated": "#ffffff",
      "--text-primary": "#201e1d", "--text-secondary": "#6c6460", "--accent-blue": "#8d593f", "--accent-green": "#3f833f",
      "--accent-purple": "#6c3f83", "--accent-gold": "#9b9b3b", "--accent-rose": "#883a54",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#dc2626",
    },
  },
  // ── Pair: Bronze Umber ──
  // Started as a swatch-extraction error under a different name (a warm
  // taupe/bronze accidentally read where a near-black was expected) — kept
  // here under its own honest name since the palette looks good on its own.
  {
    id: "dark-bronze-umber", name: "Bronze Umber", category: "standard", mode: "dark", pairId: "bronze-umber",
    preview: { bg: "#342f2c", fg: "#e6e5e5", accent: "#bd8a6b", secondary: "#3e3937" },
    vars: {
      "--bg-primary": "#342f2c", "--bg-secondary": "#3e3937", "--bg-tertiary": "#474442", "--bg-elevated": "#55504e",
      "--text-primary": "#e6e5e5", "--text-secondary": "#8b837e", "--accent-blue": "#bd8a6b", "--accent-green": "#6eb972",
      "--accent-purple": "#a46eb9", "--accent-gold": "#c6ca72", "--accent-rose": "#bf6982",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f14c4c",
    },
  },
  {
    id: "light-bronze-umber", name: "Bronze Umber", category: "standard", mode: "light", pairId: "bronze-umber",
    preview: { bg: "#f8f7f7", fg: "#201e1d", accent: "#8d5c3f", secondary: "#eeeded" },
    vars: {
      "--bg-primary": "#f8f7f7", "--bg-secondary": "#eeeded", "--bg-tertiary": "#dfdedd", "--bg-elevated": "#ffffff",
      "--text-primary": "#201e1d", "--text-secondary": "#6c6460", "--accent-blue": "#8d5c3f", "--accent-green": "#3f8342",
      "--accent-purple": "#6f3f83", "--accent-gold": "#979b3b", "--accent-rose": "#883a51",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#dc2626",
    },
  },
  // ── Pair: Eiger ──
  {
    id: "dark-eiger", name: "Eiger", category: "standard", mode: "dark", pairId: "eiger",
    preview: { bg: "#100f0f", fg: "#e6e5e5", accent: "#a25c49", secondary: "#1b1918" },
    vars: {
      "--bg-primary": "#100f0f", "--bg-secondary": "#1b1918", "--bg-tertiary": "#252322", "--bg-elevated": "#322f2f",
      "--text-primary": "#e6e5e5", "--text-secondary": "#8b817e", "--accent-blue": "#a25c49", "--accent-green": "#77b96e",
      "--accent-purple": "#976eb9", "--accent-gold": "#cabf72", "--accent-rose": "#bf6990",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f14c4c",
    },
  },
  {
    id: "light-eiger", name: "Eiger", category: "standard", mode: "light", pairId: "eiger",
    preview: { bg: "#f8f7f7", fg: "#201e1d", accent: "#9d5947", secondary: "#eeeded" },
    vars: {
      "--bg-primary": "#f8f7f7", "--bg-secondary": "#eeeded", "--bg-tertiary": "#dfdddd", "--bg-elevated": "#ffffff",
      "--text-primary": "#201e1d", "--text-secondary": "#6c6360", "--accent-blue": "#9d5947", "--accent-green": "#47833f",
      "--accent-purple": "#643f83", "--accent-gold": "#9b903b", "--accent-rose": "#883a5d",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#dc2626",
    },
  },
  // ── Pair: Indus ──
  {
    id: "dark-indus", name: "Indus", category: "standard", mode: "dark", pairId: "indus",
    preview: { bg: "#0e0f10", fg: "#e4e5e7", accent: "#6b82bd", secondary: "#18191b" },
    vars: {
      "--bg-primary": "#0e0f10", "--bg-secondary": "#18191b", "--bg-tertiary": "#212326", "--bg-elevated": "#2d2f34",
      "--text-primary": "#e4e5e7", "--text-secondary": "#7e828b", "--accent-blue": "#6b82bd", "--accent-green": "#b96e9d",
      "--accent-purple": "#6eb972", "--accent-gold": "#9472ca", "--accent-rose": "#69bbbf",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f14c4c",
    },
  },
  {
    id: "light-indus", name: "Indus", category: "standard", mode: "light", pairId: "indus",
    preview: { bg: "#f7f7f8", fg: "#1d1e20", accent: "#3f558d", secondary: "#ecedee" },
    vars: {
      "--bg-primary": "#f7f7f8", "--bg-secondary": "#ecedee", "--bg-tertiary": "#dcdde0", "--bg-elevated": "#ffffff",
      "--text-primary": "#1d1e20", "--text-secondary": "#60636c", "--accent-blue": "#3f558d", "--accent-green": "#833f69",
      "--accent-purple": "#3f8342", "--accent-gold": "#603b9b", "--accent-rose": "#3a8488",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#dc2626",
    },
  },
  // ── Pair: Carpathian ──
  {
    id: "dark-carpathian", name: "Carpathian", category: "standard", mode: "dark", pairId: "carpathian",
    preview: { bg: "#161412", fg: "#e6e6e5", accent: "#bd946b", secondary: "#201e1c" },
    vars: {
      "--bg-primary": "#161412", "--bg-secondary": "#201e1c", "--bg-tertiary": "#2a2826", "--bg-elevated": "#383532",
      "--text-primary": "#e6e6e5", "--text-secondary": "#8b857e", "--accent-blue": "#bd946b", "--accent-green": "#6eb97b",
      "--accent-purple": "#ad6eb9", "--accent-gold": "#bbca72", "--accent-rose": "#bf6977",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f14c4c",
    },
  },
  {
    id: "light-carpathian", name: "Carpathian", category: "standard", mode: "light", pairId: "carpathian",
    preview: { bg: "#f8f7f7", fg: "#201f1d", accent: "#8d663f", secondary: "#eeeded" },
    vars: {
      "--bg-primary": "#f8f7f7", "--bg-secondary": "#eeeded", "--bg-tertiary": "#dfdedd", "--bg-elevated": "#ffffff",
      "--text-primary": "#201f1d", "--text-secondary": "#6c6660", "--accent-blue": "#8d663f", "--accent-green": "#3f834a",
      "--accent-purple": "#783f83", "--accent-gold": "#8b9b3b", "--accent-rose": "#883a47",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#dc2626",
    },
  },
  // ── Pair: Narvik ──
  {
    id: "dark-narvik", name: "Narvik", category: "standard", mode: "dark", pairId: "narvik",
    preview: { bg: "#0a0b0e", fg: "#e4e5e7", accent: "#6b80bd", secondary: "#141518" },
    vars: {
      "--bg-primary": "#0a0b0e", "--bg-secondary": "#141518", "--bg-tertiary": "#1e1f23", "--bg-elevated": "#292b31",
      "--text-primary": "#e4e5e7", "--text-secondary": "#7d818c", "--accent-blue": "#6b80bd", "--accent-green": "#b96e9a",
      "--accent-purple": "#6eb975", "--accent-gold": "#9772ca", "--accent-rose": "#69b8bf",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f14c4c",
    },
  },
  {
    id: "light-narvik", name: "Narvik", category: "standard", mode: "light", pairId: "narvik",
    preview: { bg: "#f7f7f8", fg: "#1d1e21", accent: "#3f538d", secondary: "#ecedee" },
    vars: {
      "--bg-primary": "#f7f7f8", "--bg-secondary": "#ecedee", "--bg-tertiary": "#dcdde0", "--bg-elevated": "#ffffff",
      "--text-primary": "#1d1e21", "--text-secondary": "#5f636d", "--accent-blue": "#3f538d", "--accent-green": "#833f67",
      "--accent-purple": "#3f8345", "--accent-gold": "#633b9b", "--accent-rose": "#3a8188",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#dc2626",
    },
  },
  // ── Pair: Silicon ──
  {
    id: "dark-silicon", name: "Silicon", category: "standard", mode: "dark", pairId: "silicon",
    preview: { bg: "#100f0f", fg: "#e6e5e5", accent: "#a27049", secondary: "#1b1918" },
    vars: {
      "--bg-primary": "#100f0f", "--bg-secondary": "#1b1918", "--bg-tertiary": "#252422", "--bg-elevated": "#32302f",
      "--text-primary": "#e6e5e5", "--text-secondary": "#8b847e", "--accent-blue": "#a27049", "--accent-green": "#6eb977",
      "--accent-purple": "#a96eb9", "--accent-gold": "#c0ca72", "--accent-rose": "#bf697c",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f14c4c",
    },
  },
  {
    id: "light-silicon", name: "Silicon", category: "standard", mode: "light", pairId: "silicon",
    preview: { bg: "#f8f7f7", fg: "#201e1d", accent: "#7d5738", secondary: "#eeeded" },
    vars: {
      "--bg-primary": "#f8f7f7", "--bg-secondary": "#eeeded", "--bg-tertiary": "#dfdedd", "--bg-elevated": "#ffffff",
      "--text-primary": "#201e1d", "--text-secondary": "#6c6560", "--accent-blue": "#7d5738", "--accent-green": "#3f8347",
      "--accent-purple": "#743f83", "--accent-gold": "#919b3b", "--accent-rose": "#883a4b",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#dc2626",
    },
  },
  // ── Pair: Yulong ──
  {
    id: "dark-yulong", name: "Yulong", category: "standard", mode: "dark", pairId: "yulong",
    preview: { bg: "#0f0f10", fg: "#e5e5e6", accent: "#6b8abd", secondary: "#18191b" },
    vars: {
      "--bg-primary": "#0f0f10", "--bg-secondary": "#18191b", "--bg-tertiary": "#222325", "--bg-elevated": "#2f3032",
      "--text-primary": "#e5e5e6", "--text-secondary": "#7e838b", "--accent-blue": "#6b8abd", "--accent-green": "#b96ea4",
      "--accent-purple": "#72b96e", "--accent-gold": "#8c72ca", "--accent-rose": "#69bfbb",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f14c4c",
    },
  },
  {
    id: "light-yulong", name: "Yulong", category: "standard", mode: "light", pairId: "yulong",
    preview: { bg: "#f7f7f8", fg: "#1d1e20", accent: "#3f5c8d", secondary: "#ededee" },
    vars: {
      "--bg-primary": "#f7f7f8", "--bg-secondary": "#ededee", "--bg-tertiary": "#dddedf", "--bg-elevated": "#ffffff",
      "--text-primary": "#1d1e20", "--text-secondary": "#60646c", "--accent-blue": "#3f5c8d", "--accent-green": "#833f6f",
      "--accent-purple": "#42833f", "--accent-gold": "#573b9b", "--accent-rose": "#3a8884",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#dc2626",
    },
  },
  // ── Pair: Portofino ──
  {
    id: "dark-portofino", name: "Portofino", category: "standard", mode: "dark", pairId: "portofino",
    preview: { bg: "#1b2c41", fg: "#e2e5e9", accent: "#688fc0", secondary: "#2b3745" },
    vars: {
      "--bg-primary": "#1b2c41", "--bg-secondary": "#2b3745", "--bg-tertiary": "#354150", "--bg-elevated": "#3f4d5f",
      "--text-primary": "#e2e5e9", "--text-secondary": "#738396", "--accent-blue": "#688fc0", "--accent-green": "#c068ad",
      "--accent-purple": "#72c068", "--accent-gold": "#826cd0", "--accent-rose": "#62c5ba",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f14c4c",
    },
  },
  {
    id: "light-portofino", name: "Portofino", category: "standard", mode: "light", pairId: "portofino",
    preview: { bg: "#f6f7f8", fg: "#1a1e24", accent: "#3c6290", secondary: "#ebedf0" },
    vars: {
      "--bg-primary": "#f6f7f8", "--bg-secondary": "#ebedf0", "--bg-tertiary": "#d9dde3", "--bg-elevated": "#ffffff",
      "--text-primary": "#1a1e24", "--text-secondary": "#556477", "--accent-blue": "#3c6290", "--accent-green": "#893977",
      "--accent-purple": "#428939", "--accent-gold": "#4c34a2", "--accent-rose": "#348e84",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#dc2626",
    },
  },
  // ── Pair: Gondwana ──
  {
    id: "dark-gondwana", name: "Gondwana", category: "standard", mode: "dark", pairId: "gondwana",
    preview: { bg: "#11100d", fg: "#e8e6e3", accent: "#bd986b", secondary: "#1d1a16" },
    vars: {
      "--bg-primary": "#11100d", "--bg-secondary": "#1d1a16", "--bg-tertiary": "#29241f", "--bg-elevated": "#37312a",
      "--text-primary": "#e8e6e3", "--text-secondary": "#908679", "--accent-blue": "#bd986b", "--accent-green": "#6eb97f",
      "--accent-purple": "#b16eb9", "--accent-gold": "#b7ca72", "--accent-rose": "#bf6973",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f14c4c",
    },
  },
  {
    id: "light-gondwana", name: "Gondwana", category: "standard", mode: "light", pairId: "gondwana",
    preview: { bg: "#f8f7f7", fg: "#221f1b", accent: "#8d6a3f", secondary: "#efedeb" },
    vars: {
      "--bg-primary": "#f8f7f7", "--bg-secondary": "#efedeb", "--bg-tertiary": "#e1dedb", "--bg-elevated": "#ffffff",
      "--text-primary": "#221f1b", "--text-secondary": "#71675b", "--accent-blue": "#8d6a3f", "--accent-green": "#3f834e",
      "--accent-purple": "#7b3f83", "--accent-gold": "#869b3b", "--accent-rose": "#883a43",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#dc2626",
    },
  },
  // ── Pair: Pangea ──
  {
    id: "dark-pangea", name: "Pangea", category: "standard", mode: "dark", pairId: "pangea",
    preview: { bg: "#10100f", fg: "#e6e6e5", accent: "#a29d49", secondary: "#1b1b18" },
    vars: {
      "--bg-primary": "#10100f", "--bg-secondary": "#1b1b18", "--bg-tertiary": "#252522", "--bg-elevated": "#33322e",
      "--text-primary": "#e6e6e5", "--text-secondary": "#8b8a7e", "--accent-blue": "#a29d49", "--accent-green": "#6eb99c",
      "--accent-purple": "#b96ea4", "--accent-gold": "#94ca72", "--accent-rose": "#bf8169",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f14c4c",
    },
  },
  {
    id: "light-pangea", name: "Pangea", category: "standard", mode: "light", pairId: "pangea",
    preview: { bg: "#f8f8f7", fg: "#20201d", accent: "#928d41", secondary: "#eeeeed" },
    vars: {
      "--bg-primary": "#f8f8f7", "--bg-secondary": "#eeeeed", "--bg-tertiary": "#dfdfdd", "--bg-elevated": "#ffffff",
      "--text-primary": "#20201d", "--text-secondary": "#6c6b60", "--accent-blue": "#928d41", "--accent-green": "#3f8369",
      "--accent-purple": "#833f70", "--accent-gold": "#609b3b", "--accent-rose": "#88503a",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#dc2626",
    },
  },
  // ── Pair: Tasman ──
  {
    id: "dark-tasman", name: "Tasman", category: "standard", mode: "dark", pairId: "tasman",
    preview: { bg: "#0e0f11", fg: "#e4e5e7", accent: "#6b8ebd", secondary: "#17191c" },
    vars: {
      "--bg-primary": "#0e0f11", "--bg-secondary": "#17191c", "--bg-tertiary": "#202327", "--bg-elevated": "#2c3035",
      "--text-primary": "#e4e5e7", "--text-secondary": "#7c838d", "--accent-blue": "#6b8ebd", "--accent-green": "#b96ea8",
      "--accent-purple": "#76b96e", "--accent-gold": "#8772ca", "--accent-rose": "#69bfb7",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f14c4c",
    },
  },
  {
    id: "light-tasman", name: "Tasman", category: "standard", mode: "light", pairId: "tasman",
    preview: { bg: "#f7f7f8", fg: "#1c1e21", accent: "#3f608d", secondary: "#ecedee" },
    vars: {
      "--bg-primary": "#f7f7f8", "--bg-secondary": "#ecedee", "--bg-tertiary": "#dcdee0", "--bg-elevated": "#ffffff",
      "--text-primary": "#1c1e21", "--text-secondary": "#5e656e", "--accent-blue": "#3f608d", "--accent-green": "#833f73",
      "--accent-purple": "#45833f", "--accent-gold": "#523b9b", "--accent-rose": "#3a8880",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#dc2626",
    },
  },
  // ── Pair: Borasco ──
  {
    id: "dark-borasco", name: "Borasco", category: "standard", mode: "dark", pairId: "borasco",
    preview: { bg: "#0f0f10", fg: "#e5e5e6", accent: "#6b8ebd", secondary: "#18191b" },
    vars: {
      "--bg-primary": "#0f0f10", "--bg-secondary": "#18191b", "--bg-tertiary": "#222325", "--bg-elevated": "#2e3033",
      "--text-primary": "#e5e5e6", "--text-secondary": "#7e848b", "--accent-blue": "#6b8ebd", "--accent-green": "#b96ea8",
      "--accent-purple": "#76b96e", "--accent-gold": "#8772ca", "--accent-rose": "#69bfb7",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f14c4c",
    },
  },
  {
    id: "light-borasco", name: "Borasco", category: "standard", mode: "light", pairId: "borasco",
    preview: { bg: "#f7f7f8", fg: "#1d1e20", accent: "#3f608d", secondary: "#ededee" },
    vars: {
      "--bg-primary": "#f7f7f8", "--bg-secondary": "#ededee", "--bg-tertiary": "#dddedf", "--bg-elevated": "#ffffff",
      "--text-primary": "#1d1e20", "--text-secondary": "#60656c", "--accent-blue": "#3f608d", "--accent-green": "#833f73",
      "--accent-purple": "#45833f", "--accent-gold": "#523b9b", "--accent-rose": "#3a8880",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#dc2626",
    },
  },
  // ── Pair: Woolstone ──
  {
    id: "dark-woolstone", name: "Woolstone", category: "standard", mode: "dark", pairId: "woolstone",
    preview: { bg: "#0e100f", fg: "#e5e6e5", accent: "#6bbd75", secondary: "#181b18" },
    vars: {
      "--bg-primary": "#0e100f", "--bg-secondary": "#181b18", "--bg-tertiary": "#222622", "--bg-elevated": "#2e332e",
      "--text-primary": "#e5e6e5", "--text-secondary": "#7e8b80", "--accent-blue": "#6bbd75", "--accent-green": "#6e7eb9",
      "--accent-purple": "#b9916e", "--accent-gold": "#72cab8", "--accent-rose": "#97bf69",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f14c4c",
    },
  },
  {
    id: "light-woolstone", name: "Woolstone", category: "standard", mode: "light", pairId: "woolstone",
    preview: { bg: "#f7f8f7", fg: "#1d201d", accent: "#3f8d49", secondary: "#eceeed" },
    vars: {
      "--bg-primary": "#f7f8f7", "--bg-secondary": "#eceeed", "--bg-tertiary": "#dcdfdd", "--bg-elevated": "#ffffff",
      "--text-primary": "#1d201d", "--text-secondary": "#606c61", "--accent-blue": "#3f8d49", "--accent-green": "#3f4d83",
      "--accent-purple": "#835e3f", "--accent-gold": "#3b9b87", "--accent-rose": "#64883a",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#dc2626",
    },
  },
  // ── Pair: Deep Sandglow ──
  {
    id: "dark-deep-sandglow", name: "Deep Sandglow", category: "standard", mode: "dark", pairId: "deep-sandglow",
    preview: { bg: "#13100c", fg: "#e9e7e2", accent: "#d9a441", secondary: "#1f1b14" },
    vars: {
      "--bg-primary": "#13100c", "--bg-secondary": "#1f1b14", "--bg-tertiary": "#2c261c", "--bg-elevated": "#3b3426",
      "--text-primary": "#e9e7e2", "--text-secondary": "#9d8c6c", "--accent-blue": "#d9a441", "--accent-green": "#4cdb7a",
      "--accent-purple": "#d94cdb", "--accent-gold": "#b9e854", "--accent-rose": "#e1474a",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f14c4c",
    },
  },
  {
    id: "light-deep-sandglow", name: "Deep Sandglow", category: "standard", mode: "light", pairId: "deep-sandglow",
    preview: { bg: "#f9f8f6", fg: "#242019", accent: "#cc9329", secondary: "#f0eeea" },
    vars: {
      "--bg-primary": "#f9f8f6", "--bg-secondary": "#f0eeea", "--bg-tertiary": "#e3dfd9", "--bg-elevated": "#ffffff",
      "--text-primary": "#242019", "--text-secondary": "#7c6d50", "--accent-blue": "#cc9329", "--accent-green": "#20a249",
      "--accent-purple": "#a020a2", "--accent-gold": "#89bd19", "--accent-rose": "#a61b1e",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#dc2626",
    },
  },
  // ── Pair: Sedona ──
  {
    id: "dark-sedona", name: "Sedona", category: "standard", mode: "dark", pairId: "sedona",
    preview: { bg: "#130d0c", fg: "#e9e3e2", accent: "#b0533b", secondary: "#1f1614" },
    vars: {
      "--bg-primary": "#130d0c", "--bg-secondary": "#1f1614", "--bg-tertiary": "#2c1f1c", "--bg-elevated": "#3b2a26",
      "--text-primary": "#e9e3e2", "--text-secondary": "#9a786f", "--accent-blue": "#b0533b", "--accent-green": "#6bc95e",
      "--accent-purple": "#995ec9", "--accent-gold": "#d8ca64", "--accent-rose": "#cf598f",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f14c4c",
    },
  },
  {
    id: "light-sedona", name: "Sedona", category: "standard", mode: "light", pairId: "sedona",
    preview: { bg: "#f9f7f6", fg: "#241b19", accent: "#9c4a34", secondary: "#f0ecea" },
    vars: {
      "--bg-primary": "#f9f7f6", "--bg-secondary": "#f0ecea", "--bg-tertiary": "#e3dbd9", "--bg-elevated": "#ffffff",
      "--text-primary": "#241b19", "--text-secondary": "#7a5a52", "--accent-blue": "#9c4a34", "--accent-green": "#3c9130",
      "--accent-purple": "#653091", "--accent-gold": "#ab9c2b", "--accent-rose": "#962c5c",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#dc2626",
    },
  },
  // ── Pair: Charente ──
  {
    id: "dark-charente", name: "Charente", category: "standard", mode: "dark", pairId: "charente",
    preview: { bg: "#100f0f", fg: "#e6e6e5", accent: "#b6935b", secondary: "#1b1a18" },
    vars: {
      "--bg-primary": "#100f0f", "--bg-secondary": "#1b1a18", "--bg-tertiary": "#252422", "--bg-elevated": "#32312f",
      "--text-primary": "#e6e6e5", "--text-secondary": "#8b867e", "--accent-blue": "#b6935b", "--accent-green": "#6eb984",
      "--accent-purple": "#b66eb9", "--accent-gold": "#b1ca72", "--accent-rose": "#bf696d",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f14c4c",
    },
  },
  {
    id: "light-charente", name: "Charente", category: "standard", mode: "light", pairId: "charente",
    preview: { bg: "#f8f7f7", fg: "#201f1d", accent: "#a9854c", secondary: "#eeeded" },
    vars: {
      "--bg-primary": "#f8f7f7", "--bg-secondary": "#eeeded", "--bg-tertiary": "#dfdedd", "--bg-elevated": "#ffffff",
      "--text-primary": "#201f1d", "--text-secondary": "#6c6760", "--accent-blue": "#a9854c", "--accent-green": "#3f8352",
      "--accent-purple": "#7f3f83", "--accent-gold": "#809b3b", "--accent-rose": "#883a3e",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#dc2626",
    },
  },
  // ── Pair: Petra ──
  {
    id: "dark-petra", name: "Petra", category: "standard", mode: "dark", pairId: "petra",
    preview: { bg: "#130e0c", fg: "#e9e5e2", accent: "#ad673e", secondary: "#1f1814" },
    vars: {
      "--bg-primary": "#130e0c", "--bg-secondary": "#1f1814", "--bg-tertiary": "#2c221c", "--bg-elevated": "#3b2e26",
      "--text-primary": "#e9e5e2", "--text-secondary": "#997f70", "--accent-blue": "#ad673e", "--accent-green": "#61c765",
      "--accent-purple": "#a861c7", "--accent-gold": "#d2d667", "--accent-rose": "#cc5c7d",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f14c4c",
    },
  },
  {
    id: "light-petra", name: "Petra", category: "standard", mode: "light", pairId: "petra",
    preview: { bg: "#f9f7f6", fg: "#241d19", accent: "#a5623b", secondary: "#f0ecea" },
    vars: {
      "--bg-primary": "#f9f7f6", "--bg-secondary": "#f0ecea", "--bg-tertiary": "#e3dcd9", "--bg-elevated": "#ffffff",
      "--text-primary": "#241d19", "--text-secondary": "#796153", "--accent-blue": "#a5623b", "--accent-green": "#338f36",
      "--accent-purple": "#73338f", "--accent-gold": "#a4a82e", "--accent-rose": "#942e4c",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#dc2626",
    },
  },
  // ── Pair: Namib ──
  {
    id: "dark-namib", name: "Namib", category: "standard", mode: "dark", pairId: "namib",
    preview: { bg: "#130e0c", fg: "#e9e4e2", accent: "#b85932", secondary: "#1f1714" },
    vars: {
      "--bg-primary": "#130e0c", "--bg-secondary": "#1f1714", "--bg-tertiary": "#2c201c", "--bg-elevated": "#3b2c26",
      "--text-primary": "#e9e4e2", "--text-secondary": "#9d7a6c", "--accent-blue": "#b85932", "--accent-green": "#5cd157",
      "--accent-purple": "#a357d1", "--accent-gold": "#dfd95d", "--accent-rose": "#d75184",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f14c4c",
    },
  },
  {
    id: "light-namib", name: "Namib", category: "standard", mode: "light", pairId: "namib",
    preview: { bg: "#f9f7f6", fg: "#241c19", accent: "#a24e2c", secondary: "#f0ecea" },
    vars: {
      "--bg-primary": "#f9f7f6", "--bg-secondary": "#f0ecea", "--bg-tertiary": "#e3dcd9", "--bg-elevated": "#ffffff",
      "--text-primary": "#241c19", "--text-secondary": "#7c5c50", "--accent-blue": "#a24e2c", "--accent-green": "#2e9829",
      "--accent-purple": "#6e2998", "--accent-gold": "#b3ad23", "--accent-rose": "#9d2552",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#dc2626",
    },
  },
  // ── Pair: Varesine ──
  {
    id: "dark-varesine", name: "Varesine", category: "standard", mode: "dark", pairId: "varesine",
    preview: { bg: "#0d0f12", fg: "#e3e6e8", accent: "#4975a2", secondary: "#151a1e" },
    vars: {
      "--bg-primary": "#0d0f12", "--bg-secondary": "#151a1e", "--bg-tertiary": "#1e242a", "--bg-elevated": "#283039",
      "--text-primary": "#e3e6e8", "--text-secondary": "#768593", "--accent-blue": "#4975a2", "--accent-green": "#b96ead",
      "--accent-purple": "#7bb96e", "--accent-gold": "#8172ca", "--accent-rose": "#69bfb0",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f14c4c",
    },
  },
  {
    id: "light-varesine", name: "Varesine", category: "standard", mode: "light", pairId: "varesine",
    preview: { bg: "#f6f7f8", fg: "#1b1f23", accent: "#385a7c", secondary: "#ebedef" },
    vars: {
      "--bg-primary": "#f6f7f8", "--bg-secondary": "#ebedef", "--bg-tertiary": "#dadee2", "--bg-elevated": "#ffffff",
      "--text-primary": "#1b1f23", "--text-secondary": "#586674", "--accent-blue": "#385a7c", "--accent-green": "#833f78",
      "--accent-purple": "#4a833f", "--accent-gold": "#4b3b9b", "--accent-rose": "#3a887b",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#dc2626",
    },
  },
  // ── Pair: Lantau ──
  {
    id: "dark-lantau", name: "Lantau", category: "standard", mode: "dark", pairId: "lantau",
    preview: { bg: "#120f0d", fg: "#e8e6e3", accent: "#a27549", secondary: "#1e1a15" },
    vars: {
      "--bg-primary": "#120f0d", "--bg-secondary": "#1e1a15", "--bg-tertiary": "#2a241d", "--bg-elevated": "#393028",
      "--text-primary": "#e8e6e3", "--text-secondary": "#948575", "--accent-blue": "#a27549", "--accent-green": "#6eba7b",
      "--accent-purple": "#ad6eba", "--accent-gold": "#bcca72", "--accent-rose": "#bf6877",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f14c4c",
    },
  },
  {
    id: "light-lantau", name: "Lantau", category: "standard", mode: "light", pairId: "lantau",
    preview: { bg: "#f8f7f6", fg: "#231f1a", accent: "#7c5a38", secondary: "#efedeb" },
    vars: {
      "--bg-primary": "#f8f7f6", "--bg-secondary": "#efedeb", "--bg-tertiary": "#e2deda", "--bg-elevated": "#ffffff",
      "--text-primary": "#231f1a", "--text-secondary": "#756657", "--accent-blue": "#7c5a38", "--accent-green": "#3e834a",
      "--accent-purple": "#783e83", "--accent-gold": "#8c9c3a", "--accent-rose": "#883a47",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#dc2626",
    },
  },
  // ── Pair: Velocity ──
  {
    id: "dark-velocity", name: "Velocity", category: "standard", mode: "dark", pairId: "velocity",
    preview: { bg: "#0c0f13", fg: "#e2e5e9", accent: "#5d89cb", secondary: "#14181f" },
    vars: {
      "--bg-primary": "#0c0f13", "--bg-secondary": "#14181f", "--bg-tertiary": "#1c222c", "--bg-elevated": "#262e3b",
      "--text-primary": "#e2e5e9", "--text-secondary": "#6e809b", "--accent-blue": "#5d89cb", "--accent-green": "#cb5dae",
      "--accent-purple": "#65cb5d", "--accent-gold": "#8262da", "--accent-rose": "#57d1c8",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f14c4c",
    },
  },
  {
    id: "light-velocity", name: "Velocity", category: "standard", mode: "light", pairId: "velocity",
    preview: { bg: "#f6f7f9", fg: "#191e24", accent: "#315c9b", secondary: "#eaedf0" },
    vars: {
      "--bg-primary": "#f6f7f9", "--bg-secondary": "#eaedf0", "--bg-tertiary": "#d9dde3", "--bg-elevated": "#ffffff",
      "--text-primary": "#191e24", "--text-secondary": "#51627b", "--accent-blue": "#315c9b", "--accent-green": "#932f79",
      "--accent-purple": "#36932f", "--accent-gold": "#4c29ad", "--accent-rose": "#2a9890",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#dc2626",
    },
  },
  // ── Pair: Racing ──
  {
    id: "dark-racing", name: "Racing", category: "standard", mode: "dark", pairId: "racing",
    preview: { bg: "#130c0c", fg: "#e9e2e2", accent: "#d6312b", secondary: "#1f1414" },
    vars: {
      "--bg-primary": "#130c0c", "--bg-secondary": "#1f1414", "--bg-tertiary": "#2c1c1c", "--bg-elevated": "#3b2726",
      "--text-primary": "#e9e2e2", "--text-secondary": "#9d6e6c", "--accent-blue": "#d6312b", "--accent-green": "#77dc4c",
      "--accent-purple": "#814cdc", "--accent-gold": "#e9bc53", "--accent-rose": "#e246a8",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f14c4c",
    },
  },
  {
    id: "light-racing", name: "Racing", category: "standard", mode: "light", pairId: "racing",
    preview: { bg: "#f9f6f6", fg: "#241919", accent: "#cd2d28", secondary: "#f0eaea" },
    vars: {
      "--bg-primary": "#f9f6f6", "--bg-secondary": "#f0eaea", "--bg-tertiary": "#e3d9d9", "--bg-elevated": "#ffffff",
      "--text-primary": "#241919", "--text-secondary": "#7c5150", "--accent-blue": "#cd2d28", "--accent-green": "#46a21f",
      "--accent-purple": "#501fa2", "--accent-gold": "#be8d18", "--accent-rose": "#a71b73",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#dc2626",
    },
  },
  // ── Pair: Pearl ──
  {
    id: "dark-pearl", name: "Pearl", category: "standard", mode: "dark", pairId: "pearl",
    preview: { bg: "#0e0e11", fg: "#e4e4e7", accent: "#6b6bbd", secondary: "#17171c" },
    vars: {
      "--bg-primary": "#0e0e11", "--bg-secondary": "#17171c", "--bg-tertiary": "#212127", "--bg-elevated": "#2c2c35",
      "--text-primary": "#e4e4e7", "--text-secondary": "#7d7d8c", "--accent-blue": "#6b6bbd", "--accent-green": "#b96e87",
      "--accent-purple": "#6eb987", "--accent-gold": "#ad72ca", "--accent-rose": "#69a2bf",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f14c4c",
    },
  },
  {
    id: "light-pearl", name: "Pearl", category: "standard", mode: "light", pairId: "pearl",
    preview: { bg: "#dedee8", fg: "#1c1c21", accent: "#3f3f8d", secondary: "#d6d6dc" },
    vars: {
      "--bg-primary": "#dedee8", "--bg-secondary": "#d6d6dc", "--bg-tertiary": "#c6c6cd", "--bg-elevated": "#ffffff",
      "--text-primary": "#1c1c21", "--text-secondary": "#5f5f6d", "--accent-blue": "#3f3f8d", "--accent-green": "#833f56",
      "--accent-purple": "#3f8356", "--accent-gold": "#7b3b9b", "--accent-rose": "#3a6e88",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#dc2626",
    },
  },
  // ── Pair: Marine ──
  {
    id: "dark-marine", name: "Marine", category: "standard", mode: "dark", pairId: "marine",
    preview: { bg: "#0c0f13", fg: "#e2e5e9", accent: "#6390c5", secondary: "#14191f" },
    vars: {
      "--bg-primary": "#0c0f13", "--bg-secondary": "#14191f", "--bg-tertiary": "#1c232c", "--bg-elevated": "#26303b",
      "--text-primary": "#e2e5e9", "--text-secondary": "#718398", "--accent-blue": "#6390c5", "--accent-green": "#c563b0",
      "--accent-purple": "#70c563", "--accent-gold": "#7f68d4", "--accent-rose": "#5ecabc",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f14c4c",
    },
  },
  {
    id: "light-marine", name: "Marine", category: "standard", mode: "light", pairId: "marine",
    preview: { bg: "#f6f7f9", fg: "#191e24", accent: "#386294", secondary: "#eaedf0" },
    vars: {
      "--bg-primary": "#f6f7f9", "--bg-secondary": "#eaedf0", "--bg-tertiary": "#d9dde3", "--bg-elevated": "#ffffff",
      "--text-primary": "#191e24", "--text-secondary": "#536579", "--accent-blue": "#386294", "--accent-green": "#8d357b",
      "--accent-purple": "#408d35", "--accent-gold": "#4830a6", "--accent-rose": "#309285",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#dc2626",
    },
  },
  // ── Pair: Balmoral ──
  {
    id: "dark-balmoral", name: "Balmoral", category: "standard", mode: "dark", pairId: "balmoral",
    preview: { bg: "#0e110e", fg: "#e5e7e4", accent: "#7abd6b", secondary: "#181c17" },
    vars: {
      "--bg-primary": "#0e110e", "--bg-secondary": "#181c17", "--bg-tertiary": "#222721", "--bg-elevated": "#2e342d",
      "--text-primary": "#e5e7e4", "--text-secondary": "#808c7e", "--accent-blue": "#7abd6b", "--accent-green": "#6e95b9",
      "--accent-purple": "#b97a6e", "--accent-gold": "#72ca9d", "--accent-rose": "#b1bf69",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f14c4c",
    },
  },
  {
    id: "light-balmoral", name: "Balmoral", category: "standard", mode: "light", pairId: "balmoral",
    preview: { bg: "#f7f8f7", fg: "#1d211d", accent: "#4d8d3f", secondary: "#eceeec" },
    vars: {
      "--bg-primary": "#f7f8f7", "--bg-secondary": "#eceeec", "--bg-tertiary": "#dde0dc", "--bg-elevated": "#ffffff",
      "--text-primary": "#1d211d", "--text-secondary": "#626d5f", "--accent-blue": "#4d8d3f", "--accent-green": "#3f6283",
      "--accent-purple": "#834a3f", "--accent-gold": "#3b9b6a", "--accent-rose": "#7c883a",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#dc2626",
    },
  },
  // ── Restored pairs: the 10 Rivian-inspired, 6 supercar-inspired, and 7
  // iPhone-inspired palettes removed in the brand-name purge — same colors,
  // renamed with no reference to cars or phones (category "standard", plain
  // ids), per the user's explicit ask for this round.

  // ── Pair: Denim (was Rivian Blue) ──
  {
    id: "dark-denim", name: "Denim", category: "standard", mode: "dark", pairId: "denim",
    preview: { bg: "#0b1628", fg: "#d4dce8", accent: "#3d7bce", secondary: "#122040" },
    vars: {
      "--bg-primary": "#0b1628", "--bg-secondary": "#122040", "--bg-tertiary": "#1a2d52", "--bg-elevated": "#233a64",
      "--text-primary": "#d4dce8", "--text-secondary": "#7a90ad", "--accent-blue": "#3d7bce", "--accent-green": "#4caf82",
      "--accent-purple": "#9b8ec7", "--accent-gold": "#e5b84c", "--accent-rose": "#e06070",
      "--border-color": "rgba(61, 123, 206, 0.12)", "--error-color": "#e06070",
    },
  },
  {
    id: "light-denim", name: "Denim", category: "standard", mode: "light", pairId: "denim",
    preview: { bg: "#f4f6f9", fg: "#1a2a42", accent: "#2a5fa0", secondary: "#e6ebf2" },
    vars: {
      "--bg-primary": "#f4f6f9", "--bg-secondary": "#e6ebf2", "--bg-tertiary": "#d4dce8", "--bg-elevated": "#ffffff",
      "--text-primary": "#1a2a42", "--text-secondary": "#5a6e85", "--accent-blue": "#2a5fa0", "--accent-green": "#2d8a5e",
      "--accent-purple": "#6b5ea0", "--accent-gold": "#b08a20", "--accent-rose": "#c44858",
      "--border-color": "rgba(42, 95, 160, 0.10)", "--error-color": "#c44858",
    },
  },
  // ── Pair: Juniper (was Rivian Forest) ──
  {
    id: "dark-juniper", name: "Juniper", category: "standard", mode: "dark", pairId: "juniper",
    preview: { bg: "#0e1a15", fg: "#d0ddd4", accent: "#4a8c6a", secondary: "#162820" },
    vars: {
      "--bg-primary": "#0e1a15", "--bg-secondary": "#162820", "--bg-tertiary": "#1e352b", "--bg-elevated": "#274236",
      "--text-primary": "#d0ddd4", "--text-secondary": "#7a9988", "--accent-blue": "#4a8c6a", "--accent-green": "#5ebd88",
      "--accent-purple": "#a28db5", "--accent-gold": "#d4a855", "--accent-rose": "#d46a5a",
      "--border-color": "rgba(74, 140, 106, 0.12)", "--error-color": "#d46a5a",
    },
  },
  {
    id: "light-juniper", name: "Juniper", category: "standard", mode: "light", pairId: "juniper",
    preview: { bg: "#f5f0e8", fg: "#2a2820", accent: "#3d7a58", secondary: "#e8e0d2" },
    vars: {
      "--bg-primary": "#f5f0e8", "--bg-secondary": "#e8e0d2", "--bg-tertiary": "#d8cebc", "--bg-elevated": "#fdf8f0",
      "--text-primary": "#2a2820", "--text-secondary": "#6a6456", "--accent-blue": "#3d7a58", "--accent-green": "#4a9060",
      "--accent-purple": "#7a6690", "--accent-gold": "#a08228", "--accent-rose": "#b84a40",
      "--border-color": "rgba(61, 122, 88, 0.10)", "--error-color": "#b84a40",
    },
  },
  // ── Pair: Granite (was Rivian Granite) ──
  {
    id: "dark-granite", name: "Granite", category: "standard", mode: "dark", pairId: "granite",
    preview: { bg: "#161514", fg: "#d5d0ca", accent: "#a09080", secondary: "#221f1d" },
    vars: {
      "--bg-primary": "#161514", "--bg-secondary": "#221f1d", "--bg-tertiary": "#2e2a27", "--bg-elevated": "#3a3532",
      "--text-primary": "#d5d0ca", "--text-secondary": "#8a8278", "--accent-blue": "#a09080", "--accent-green": "#7aaa6c",
      "--accent-purple": "#b098b8", "--accent-gold": "#d4a855", "--accent-rose": "#cc6a5a",
      "--border-color": "rgba(160, 144, 128, 0.12)", "--error-color": "#cc6a5a",
    },
  },
  {
    id: "light-granite", name: "Granite", category: "standard", mode: "light", pairId: "granite",
    preview: { bg: "#f0eeec", fg: "#2a2624", accent: "#6a6058", secondary: "#e2dedb" },
    vars: {
      "--bg-primary": "#f0eeec", "--bg-secondary": "#e2dedb", "--bg-tertiary": "#d0ccc7", "--bg-elevated": "#faf8f6",
      "--text-primary": "#2a2624", "--text-secondary": "#6e665e", "--accent-blue": "#6a6058", "--accent-green": "#508a48",
      "--accent-purple": "#7a6880", "--accent-gold": "#9a7a28", "--accent-rose": "#aa4a40",
      "--border-color": "rgba(106, 96, 88, 0.10)", "--error-color": "#aa4a40",
    },
  },
  // ── Pair: Nightfall (was Rivian Midnight) ──
  {
    id: "dark-nightfall", name: "Nightfall", category: "standard", mode: "dark", pairId: "nightfall",
    preview: { bg: "#08090e", fg: "#cdd2dc", accent: "#5a8aaa", secondary: "#10121a" },
    vars: {
      "--bg-primary": "#08090e", "--bg-secondary": "#10121a", "--bg-tertiary": "#181c28", "--bg-elevated": "#222838",
      "--text-primary": "#cdd2dc", "--text-secondary": "#6a7488", "--accent-blue": "#5a8aaa", "--accent-green": "#4aaa80",
      "--accent-purple": "#8a80b8", "--accent-gold": "#ccaa44", "--accent-rose": "#d85860",
      "--border-color": "rgba(90, 138, 170, 0.10)", "--error-color": "#d85860",
    },
  },
  {
    id: "light-nightfall", name: "Nightfall", category: "standard", mode: "light", pairId: "nightfall",
    preview: { bg: "#f0f6f8", fg: "#1a2830", accent: "#3a7a94", secondary: "#deeef4" },
    vars: {
      "--bg-primary": "#f0f6f8", "--bg-secondary": "#deeef4", "--bg-tertiary": "#c8dee8", "--bg-elevated": "#fafcfd",
      "--text-primary": "#1a2830", "--text-secondary": "#4a6878", "--accent-blue": "#3a7a94", "--accent-green": "#2a8a60",
      "--accent-purple": "#6a6090", "--accent-gold": "#a08828", "--accent-rose": "#c04850",
      "--border-color": "rgba(58, 122, 148, 0.10)", "--error-color": "#c04850",
    },
  },
  // ── Pair: Canyon (was Rivian Canyon) ──
  {
    id: "dark-canyon", name: "Canyon", category: "standard", mode: "dark", pairId: "canyon",
    preview: { bg: "#140c0a", fg: "#e0d0c8", accent: "#b85a42", secondary: "#221410" },
    vars: {
      "--bg-primary": "#140c0a", "--bg-secondary": "#221410", "--bg-tertiary": "#30201a", "--bg-elevated": "#3e2c24",
      "--text-primary": "#e0d0c8", "--text-secondary": "#a08878", "--accent-blue": "#b85a42", "--accent-green": "#6aaa58",
      "--accent-purple": "#a87898", "--accent-gold": "#d4a040", "--accent-rose": "#d85040",
      "--border-color": "rgba(184, 90, 66, 0.14)", "--error-color": "#d85040",
    },
  },
  {
    id: "light-canyon", name: "Canyon", category: "standard", mode: "light", pairId: "canyon",
    preview: { bg: "#f8f2ec", fg: "#2e1e18", accent: "#984838", secondary: "#ecddd0" },
    vars: {
      "--bg-primary": "#f8f2ec", "--bg-secondary": "#ecddd0", "--bg-tertiary": "#dccabc", "--bg-elevated": "#fffaf5",
      "--text-primary": "#2e1e18", "--text-secondary": "#7a5e50", "--accent-blue": "#984838", "--accent-green": "#4a8a3e",
      "--accent-purple": "#804a6a", "--accent-gold": "#a07820", "--accent-rose": "#b83830",
      "--border-color": "rgba(152, 72, 56, 0.10)", "--error-color": "#b83830",
    },
  },
  // ── Pair: Meadow (was Rivian Launch) ──
  {
    id: "dark-meadow", name: "Meadow", category: "standard", mode: "dark", pairId: "meadow",
    preview: { bg: "#0a140e", fg: "#d0e0d4", accent: "#66cc6a", secondary: "#142218" },
    vars: {
      "--bg-primary": "#0a140e", "--bg-secondary": "#142218", "--bg-tertiary": "#1e3024", "--bg-elevated": "#283e30",
      "--text-primary": "#d0e0d4", "--text-secondary": "#7aa088", "--accent-blue": "#66cc6a", "--accent-green": "#66cc6a",
      "--accent-purple": "#a090c0", "--accent-gold": "#ccb040", "--accent-rose": "#e05858",
      "--border-color": "rgba(76, 175, 80, 0.14)", "--error-color": "#e05858",
    },
  },
  {
    id: "light-meadow", name: "Meadow", category: "standard", mode: "light", pairId: "meadow",
    preview: { bg: "#f2f6f2", fg: "#1a2420", accent: "#2e8a38", secondary: "#e0eae2" },
    vars: {
      "--bg-primary": "#f2f6f2", "--bg-secondary": "#e0eae2", "--bg-tertiary": "#cddcd0", "--bg-elevated": "#fafcfa",
      "--text-primary": "#1a2420", "--text-secondary": "#4a6a52", "--accent-blue": "#2e8a38", "--accent-green": "#3aa040",
      "--accent-purple": "#6a5a8a", "--accent-gold": "#8a7a18", "--accent-rose": "#b84040",
      "--border-color": "rgba(46, 138, 56, 0.10)", "--error-color": "#b84040",
    },
  },
  // ── Pair: Cove (was Rivian Catalina) ──
  {
    id: "dark-cove", name: "Cove", category: "standard", mode: "dark", pairId: "cove",
    preview: { bg: "#0a1418", fg: "#ccdce0", accent: "#3a9aaa", secondary: "#122028" },
    vars: {
      "--bg-primary": "#0a1418", "--bg-secondary": "#122028", "--bg-tertiary": "#1a2e38", "--bg-elevated": "#223c48",
      "--text-primary": "#ccdce0", "--text-secondary": "#6a8a94", "--accent-blue": "#3a9aaa", "--accent-green": "#4ab888",
      "--accent-purple": "#8a88c0", "--accent-gold": "#d0a848", "--accent-rose": "#d86068",
      "--border-color": "rgba(58, 154, 170, 0.12)", "--error-color": "#d86068",
    },
  },
  {
    id: "light-cove", name: "Cove", category: "standard", mode: "light", pairId: "cove",
    preview: { bg: "#f2f8f8", fg: "#182828", accent: "#2a808e", secondary: "#dceef0" },
    vars: {
      "--bg-primary": "#f2f8f8", "--bg-secondary": "#dceef0", "--bg-tertiary": "#c6e0e4", "--bg-elevated": "#fafefe",
      "--text-primary": "#182828", "--text-secondary": "#486a70", "--accent-blue": "#2a808e", "--accent-green": "#2a9068",
      "--accent-purple": "#5a6090", "--accent-gold": "#98841e", "--accent-rose": "#b84850",
      "--border-color": "rgba(42, 128, 142, 0.10)", "--error-color": "#b84850",
    },
  },
  // ── Pair: Storm (was Rivian Storm) ──
  {
    id: "dark-storm", name: "Storm", category: "standard", mode: "dark", pairId: "storm",
    preview: { bg: "#0c1218", fg: "#ccd4dc", accent: "#4a6a88", secondary: "#141e2a" },
    vars: {
      "--bg-primary": "#0c1218", "--bg-secondary": "#141e2a", "--bg-tertiary": "#1e2c3c", "--bg-elevated": "#283a4e",
      "--text-primary": "#ccd4dc", "--text-secondary": "#6e8098", "--accent-blue": "#4a6a88", "--accent-green": "#58a878",
      "--accent-purple": "#8878a8", "--accent-gold": "#c8a44a", "--accent-rose": "#cc5860",
      "--border-color": "rgba(74, 106, 136, 0.12)", "--error-color": "#cc5860",
    },
  },
  {
    id: "light-storm", name: "Storm", category: "standard", mode: "light", pairId: "storm",
    preview: { bg: "#f0f2f4", fg: "#1e2830", accent: "#3a5a72", secondary: "#e0e4e8" },
    vars: {
      "--bg-primary": "#f0f2f4", "--bg-secondary": "#e0e4e8", "--bg-tertiary": "#ccd2d8", "--bg-elevated": "#fafbfc",
      "--text-primary": "#1e2830", "--text-secondary": "#546878", "--accent-blue": "#3a5a72", "--accent-green": "#388a58",
      "--accent-purple": "#605880", "--accent-gold": "#8a7a1e", "--accent-rose": "#a84448",
      "--border-color": "rgba(58, 90, 114, 0.10)", "--error-color": "#a84448",
    },
  },
  // ── Pair: Halfmoon (was Rivian Halfmoon) ──
  {
    id: "dark-halfmoon", name: "Halfmoon", category: "standard", mode: "dark", pairId: "halfmoon",
    preview: { bg: "#121210", fg: "#d2d0cc", accent: "#8a8478", secondary: "#1e1c1a" },
    vars: {
      "--bg-primary": "#121210", "--bg-secondary": "#1e1c1a", "--bg-tertiary": "#2a2826", "--bg-elevated": "#363432",
      "--text-primary": "#d2d0cc", "--text-secondary": "#8a8680", "--accent-blue": "#8a8478", "--accent-green": "#6ea868",
      "--accent-purple": "#a890b0", "--accent-gold": "#c8a448", "--accent-rose": "#c86058",
      "--border-color": "rgba(138, 132, 120, 0.12)", "--error-color": "#c86058",
    },
  },
  {
    id: "light-halfmoon", name: "Halfmoon", category: "standard", mode: "light", pairId: "halfmoon",
    preview: { bg: "#f0eeec", fg: "#242220", accent: "#6a6460", secondary: "#e0dcda" },
    vars: {
      "--bg-primary": "#f0eeec", "--bg-secondary": "#e0dcda", "--bg-tertiary": "#cec8c4", "--bg-elevated": "#faf8f6",
      "--text-primary": "#242220", "--text-secondary": "#645e58", "--accent-blue": "#6a6460", "--accent-green": "#488a40",
      "--accent-purple": "#6a5878", "--accent-gold": "#8a7a20", "--accent-rose": "#a84038",
      "--border-color": "rgba(106, 100, 96, 0.10)", "--error-color": "#a84038",
    },
  },
  // ── Pair: Borealis (was Rivian Borealis) ──
  {
    id: "dark-borealis", name: "Borealis", category: "standard", mode: "dark", pairId: "borealis",
    preview: { bg: "#0a1210", fg: "#d0e0d8", accent: "#38a088", secondary: "#142220" },
    vars: {
      "--bg-primary": "#0a1210", "--bg-secondary": "#142220", "--bg-tertiary": "#1c302c", "--bg-elevated": "#243e38",
      "--text-primary": "#d0e0d8", "--text-secondary": "#6a9a8c", "--accent-blue": "#38a088", "--accent-green": "#50c898",
      "--accent-purple": "#8888c0", "--accent-gold": "#c8b048", "--accent-rose": "#d06060",
      "--border-color": "rgba(56, 160, 136, 0.14)", "--error-color": "#d06060",
    },
  },
  {
    id: "light-borealis", name: "Borealis", category: "standard", mode: "light", pairId: "borealis",
    preview: { bg: "#f4f8f6", fg: "#1a2822", accent: "#2a8070", secondary: "#e0ece8" },
    vars: {
      "--bg-primary": "#f4f8f6", "--bg-secondary": "#e0ece8", "--bg-tertiary": "#ccdcd6", "--bg-elevated": "#fafefc",
      "--text-primary": "#1a2822", "--text-secondary": "#466a60", "--accent-blue": "#2a8070", "--accent-green": "#38a06a",
      "--accent-purple": "#5a5a88", "--accent-gold": "#8a8020", "--accent-rose": "#b04040",
      "--border-color": "rgba(42, 128, 112, 0.10)", "--error-color": "#b04040",
    },
  },
  // ── Pair: Graphite (was Black Titanium) ──
  {
    id: "dark-graphite", name: "Graphite", category: "standard", mode: "dark", pairId: "graphite",
    preview: { bg: "#0e0e10", fg: "#d6d4d0", accent: "#8a8680", secondary: "#1a1a1e" },
    vars: {
      "--bg-primary": "#0e0e10", "--bg-secondary": "#1a1a1e", "--bg-tertiary": "#24242a", "--bg-elevated": "#2e2e36",
      "--text-primary": "#d6d4d0", "--text-secondary": "#807c76", "--accent-blue": "#8a8680", "--accent-green": "#68b070",
      "--accent-purple": "#a090b8", "--accent-gold": "#d0a848", "--accent-rose": "#d46058",
      "--border-color": "rgba(138, 134, 128, 0.10)", "--error-color": "#d46058",
    },
  },
  {
    id: "light-graphite", name: "Graphite", category: "standard", mode: "light", pairId: "graphite",
    preview: { bg: "#f6f4f2", fg: "#22201e", accent: "#706c68", secondary: "#eae8e4" },
    vars: {
      "--bg-primary": "#f6f4f2", "--bg-secondary": "#eae8e4", "--bg-tertiary": "#d8d4d0", "--bg-elevated": "#fcfaf8",
      "--text-primary": "#22201e", "--text-secondary": "#5e5a56", "--accent-blue": "#706c68", "--accent-green": "#488a42",
      "--accent-purple": "#685c78", "--accent-gold": "#8a7a1e", "--accent-rose": "#a84038",
      "--border-color": "rgba(112, 108, 104, 0.08)", "--error-color": "#a84038",
    },
  },
  // ── Pair: Slate (was Blue Titanium) ──
  {
    id: "dark-slate", name: "Slate", category: "standard", mode: "dark", pairId: "slate",
    preview: { bg: "#0c1018", fg: "#d0d4dc", accent: "#5a7898", secondary: "#141c28" },
    vars: {
      "--bg-primary": "#0c1018", "--bg-secondary": "#141c28", "--bg-tertiary": "#1c2838", "--bg-elevated": "#263448",
      "--text-primary": "#d0d4dc", "--text-secondary": "#6a7a94", "--accent-blue": "#5a7898", "--accent-green": "#50a872",
      "--accent-purple": "#8a80b0", "--accent-gold": "#c8a040", "--accent-rose": "#cc5860",
      "--border-color": "rgba(90, 120, 152, 0.10)", "--error-color": "#cc5860",
    },
  },
  {
    id: "light-slate", name: "Slate", category: "standard", mode: "light", pairId: "slate",
    preview: { bg: "#f2f0ee", fg: "#222020", accent: "#5a6878", secondary: "#e4e2de" },
    vars: {
      "--bg-primary": "#f2f0ee", "--bg-secondary": "#e4e2de", "--bg-tertiary": "#d2cec8", "--bg-elevated": "#faf8f6",
      "--text-primary": "#222020", "--text-secondary": "#5c5854", "--accent-blue": "#5a6878", "--accent-green": "#3e8a48",
      "--accent-purple": "#645a78", "--accent-gold": "#8a7820", "--accent-rose": "#a84040",
      "--border-color": "rgba(90, 104, 120, 0.08)", "--error-color": "#a84040",
    },
  },
  // ── Pair: Dune (was Desert Titanium) ──
  {
    id: "dark-dune", name: "Dune", category: "standard", mode: "dark", pairId: "dune",
    preview: { bg: "#12100e", fg: "#d8d0c8", accent: "#a89070", secondary: "#1e1a16" },
    vars: {
      "--bg-primary": "#12100e", "--bg-secondary": "#1e1a16", "--bg-tertiary": "#2a2620", "--bg-elevated": "#36302a",
      "--text-primary": "#d8d0c8", "--text-secondary": "#8a8274", "--accent-blue": "#a89070", "--accent-green": "#6aa860",
      "--accent-purple": "#a890a8", "--accent-gold": "#d0a44a", "--accent-rose": "#cc6050",
      "--border-color": "rgba(168, 144, 112, 0.12)", "--error-color": "#cc6050",
    },
  },
  {
    id: "light-dune", name: "Dune", category: "standard", mode: "light", pairId: "dune",
    preview: { bg: "#f6f0ea", fg: "#282218", accent: "#886838", secondary: "#ece2d6" },
    vars: {
      "--bg-primary": "#f6f0ea", "--bg-secondary": "#ece2d6", "--bg-tertiary": "#dcd0c0", "--bg-elevated": "#fef8f0",
      "--text-primary": "#282218", "--text-secondary": "#6e6050", "--accent-blue": "#886838", "--accent-green": "#508838",
      "--accent-purple": "#7a6680", "--accent-gold": "#9a7a18", "--accent-rose": "#aa4438",
      "--border-color": "rgba(136, 104, 56, 0.10)", "--error-color": "#aa4438",
    },
  },
  // ── Pair: Ultramarine ──
  {
    id: "dark-ultramarine", name: "Ultramarine", category: "standard", mode: "dark", pairId: "ultramarine",
    preview: { bg: "#0a0c1a", fg: "#d0d4e8", accent: "#4860d0", secondary: "#141840" },
    vars: {
      "--bg-primary": "#0a0c1a", "--bg-secondary": "#141840", "--bg-tertiary": "#1c2258", "--bg-elevated": "#262e68",
      "--text-primary": "#d0d4e8", "--text-secondary": "#6a70b0", "--accent-blue": "#4860d0", "--accent-green": "#48b878",
      "--accent-purple": "#8a6ae0", "--accent-gold": "#dab040", "--accent-rose": "#e05868",
      "--border-color": "rgba(72, 96, 208, 0.12)", "--error-color": "#e05868",
    },
  },
  {
    id: "light-ultramarine", name: "Ultramarine", category: "standard", mode: "light", pairId: "ultramarine",
    preview: { bg: "#f2f2fa", fg: "#181830", accent: "#3040b0", secondary: "#e2e2f0" },
    vars: {
      "--bg-primary": "#f2f2fa", "--bg-secondary": "#e2e2f0", "--bg-tertiary": "#d0d0e2", "--bg-elevated": "#fafaff",
      "--text-primary": "#181830", "--text-secondary": "#4a4a78", "--accent-blue": "#3040b0", "--accent-green": "#2a8a50",
      "--accent-purple": "#5a3aaa", "--accent-gold": "#9a8018", "--accent-rose": "#b83848",
      "--border-color": "rgba(48, 64, 176, 0.10)", "--error-color": "#b83848",
    },
  },
  // ── Pair: Teal (was iPhone Teal) ──
  {
    id: "dark-teal", name: "Teal", category: "standard", mode: "dark", pairId: "teal",
    preview: { bg: "#0a1214", fg: "#d0dce0", accent: "#3a9aa0", secondary: "#121e22" },
    vars: {
      "--bg-primary": "#0a1214", "--bg-secondary": "#121e22", "--bg-tertiary": "#1a2c32", "--bg-elevated": "#223a42",
      "--text-primary": "#d0dce0", "--text-secondary": "#6a8a90", "--accent-blue": "#3a9aa0", "--accent-green": "#48c088",
      "--accent-purple": "#8088c0", "--accent-gold": "#c8aa40", "--accent-rose": "#d06060",
      "--border-color": "rgba(58, 154, 160, 0.12)", "--error-color": "#d06060",
    },
  },
  {
    id: "light-teal", name: "Teal", category: "standard", mode: "light", pairId: "teal",
    preview: { bg: "#f0f8f8", fg: "#182828", accent: "#288088", secondary: "#dceef0" },
    vars: {
      "--bg-primary": "#f0f8f8", "--bg-secondary": "#dceef0", "--bg-tertiary": "#c8e0e2", "--bg-elevated": "#fafefe",
      "--text-primary": "#182828", "--text-secondary": "#466a6e", "--accent-blue": "#288088", "--accent-green": "#2a8a5e",
      "--accent-purple": "#5a6088", "--accent-gold": "#8a8018", "--accent-rose": "#b04848",
      "--border-color": "rgba(40, 128, 136, 0.10)", "--error-color": "#b04848",
    },
  },
  // ── Pair: Blossom (was iPhone Pink) ──
  {
    id: "dark-blossom", name: "Blossom", category: "standard", mode: "dark", pairId: "blossom",
    preview: { bg: "#140c10", fg: "#e0d4d8", accent: "#c06888", secondary: "#221420" },
    vars: {
      "--bg-primary": "#140c10", "--bg-secondary": "#221420", "--bg-tertiary": "#301e2c", "--bg-elevated": "#3e283a",
      "--text-primary": "#e0d4d8", "--text-secondary": "#9a7888", "--accent-blue": "#c06888", "--accent-green": "#5ab870",
      "--accent-purple": "#b080c8", "--accent-gold": "#d0a448", "--accent-rose": "#e05070",
      "--border-color": "rgba(192, 104, 136, 0.12)", "--error-color": "#e05070",
    },
  },
  {
    id: "light-blossom", name: "Blossom", category: "standard", mode: "light", pairId: "blossom",
    preview: { bg: "#f8f0f2", fg: "#2a1e22", accent: "#a04868", secondary: "#f0e0e4" },
    vars: {
      "--bg-primary": "#f8f0f2", "--bg-secondary": "#f0e0e4", "--bg-tertiary": "#e0ccd2", "--bg-elevated": "#fef8fa",
      "--text-primary": "#2a1e22", "--text-secondary": "#785060", "--accent-blue": "#a04868", "--accent-green": "#3e8a48",
      "--accent-purple": "#884880", "--accent-gold": "#9a7a20", "--accent-rose": "#c03050",
      "--border-color": "rgba(160, 72, 104, 0.10)", "--error-color": "#c03050",
    },
  },
  // ── Pair: Willow (was iPhone Green) ──
  {
    id: "dark-willow", name: "Willow", category: "standard", mode: "dark", pairId: "willow",
    preview: { bg: "#0c120e", fg: "#d0dcd4", accent: "#58a868", secondary: "#142018" },
    vars: {
      "--bg-primary": "#0c120e", "--bg-secondary": "#142018", "--bg-tertiary": "#1e2e22", "--bg-elevated": "#283c2e",
      "--text-primary": "#d0dcd4", "--text-secondary": "#6e9878", "--accent-blue": "#58a868", "--accent-green": "#68cc78",
      "--accent-purple": "#9888b8", "--accent-gold": "#c8aa38", "--accent-rose": "#d06058",
      "--border-color": "rgba(88, 168, 104, 0.12)", "--error-color": "#d06058",
    },
  },
  {
    id: "light-willow", name: "Willow", category: "standard", mode: "light", pairId: "willow",
    preview: { bg: "#f8f6ee", fg: "#22201a", accent: "#a09020", secondary: "#eeeadc" },
    vars: {
      "--bg-primary": "#f8f6ee", "--bg-secondary": "#eeeadc", "--bg-tertiary": "#dcd6c4", "--bg-elevated": "#fefcf4",
      "--text-primary": "#22201a", "--text-secondary": "#6a6450", "--accent-blue": "#a09020", "--accent-green": "#4a8a38",
      "--accent-purple": "#6e5e88", "--accent-gold": "#a08a18", "--accent-rose": "#b04038",
      "--border-color": "rgba(160, 144, 32, 0.10)", "--error-color": "#b04038",
    },
  },
  // ── Pair: Argent (was Pagani) ──
  {
    id: "dark-argent", name: "Argent", category: "standard", mode: "dark", pairId: "argent",
    preview: { bg: "#0a0c10", fg: "#c8cdd8", accent: "#7eb8da", secondary: "#141820" },
    vars: {
      "--bg-primary": "#0a0c10", "--bg-secondary": "#141820", "--bg-tertiary": "#1c222e", "--bg-elevated": "#242c3a",
      "--text-primary": "#c8cdd8", "--text-secondary": "#6a7488", "--accent-blue": "#7eb8da", "--accent-green": "#6ecfb0",
      "--accent-purple": "#9ca8c0", "--accent-gold": "#b8c4d8", "--accent-rose": "#d47888",
      "--border-color": "rgba(126, 184, 218, 0.08)", "--error-color": "#d47888",
    },
  },
  {
    id: "light-argent", name: "Argent", category: "standard", mode: "light", pairId: "argent",
    preview: { bg: "#f4f6f9", fg: "#0e1218", accent: "#3a7ca5", secondary: "#e4e8ee" },
    vars: {
      "--bg-primary": "#f4f6f9", "--bg-secondary": "#e4e8ee", "--bg-tertiary": "#d0d6e0", "--bg-elevated": "#ffffff",
      "--text-primary": "#0e1218", "--text-secondary": "#5a6478", "--accent-blue": "#3a7ca5", "--accent-green": "#3a9a7c",
      "--accent-purple": "#6878a0", "--accent-gold": "#5a7a98", "--accent-rose": "#a84858",
      "--border-color": "rgba(58, 124, 165, 0.10)", "--error-color": "#a84858",
    },
  },
  // ── Pair: Saffron (was Lamborghini) ──
  {
    id: "dark-saffron", name: "Saffron", category: "standard", mode: "dark", pairId: "saffron",
    preview: { bg: "#0c0a00", fg: "#f0e8c8", accent: "#e8c820", secondary: "#1c1800" },
    vars: {
      "--bg-primary": "#0c0a00", "--bg-secondary": "#1c1800", "--bg-tertiary": "#282200", "--bg-elevated": "#342c08",
      "--text-primary": "#f0e8c8", "--text-secondary": "#8a8260", "--accent-blue": "#e8c820", "--accent-green": "#88c828",
      "--accent-purple": "#c8a830", "--accent-gold": "#e8c820", "--accent-rose": "#e84830",
      "--border-color": "rgba(232, 200, 32, 0.10)", "--error-color": "#e84830",
    },
  },
  {
    id: "light-saffron", name: "Saffron", category: "standard", mode: "light", pairId: "saffron",
    preview: { bg: "#fefbe8", fg: "#1a1800", accent: "#b89b00", secondary: "#f5f0c8" },
    vars: {
      "--bg-primary": "#fefbe8", "--bg-secondary": "#f5f0c8", "--bg-tertiary": "#e8e2a8", "--bg-elevated": "#fffef0",
      "--text-primary": "#1a1800", "--text-secondary": "#6a6230", "--accent-blue": "#b89b00", "--accent-green": "#5a8a10",
      "--accent-purple": "#8a7a18", "--accent-gold": "#b89b00", "--accent-rose": "#c03020",
      "--border-color": "rgba(184, 155, 0, 0.12)", "--error-color": "#c03020",
    },
  },
  // ── Pair: Emerald (was Porsche) ──
  {
    id: "dark-emerald", name: "Emerald", category: "standard", mode: "dark", pairId: "emerald",
    preview: { bg: "#08100c", fg: "#d0e0d4", accent: "#2e8b57", secondary: "#142018" },
    vars: {
      "--bg-primary": "#08100c", "--bg-secondary": "#142018", "--bg-tertiary": "#1c2e24", "--bg-elevated": "#243a2e",
      "--text-primary": "#d0e0d4", "--text-secondary": "#6a8a74", "--accent-blue": "#2e8b57", "--accent-green": "#4ade80",
      "--accent-purple": "#58a878", "--accent-gold": "#c8b830", "--accent-rose": "#d06858",
      "--border-color": "rgba(46, 139, 87, 0.10)", "--error-color": "#d06858",
    },
  },
  {
    id: "light-emerald", name: "Emerald", category: "standard", mode: "light", pairId: "emerald",
    preview: { bg: "#f2f7f4", fg: "#0a1a10", accent: "#1a6b3c", secondary: "#dceee4" },
    vars: {
      "--bg-primary": "#f2f7f4", "--bg-secondary": "#dceee4", "--bg-tertiary": "#c4e0cc", "--bg-elevated": "#ffffff",
      "--text-primary": "#0a1a10", "--text-secondary": "#4a6a54", "--accent-blue": "#1a6b3c", "--accent-green": "#2a9a58",
      "--accent-purple": "#3a7a50", "--accent-gold": "#9a8a10", "--accent-rose": "#a85040",
      "--border-color": "rgba(26, 107, 60, 0.10)", "--error-color": "#a85040",
    },
  },
  // ── Pair: Indigo (was Bugatti) ──
  {
    id: "dark-indigo", name: "Indigo", category: "standard", mode: "dark", pairId: "indigo",
    preview: { bg: "#040810", fg: "#c0c8e0", accent: "#1e3a8a", secondary: "#0c1428" },
    vars: {
      "--bg-primary": "#040810", "--bg-secondary": "#0c1428", "--bg-tertiary": "#142040", "--bg-elevated": "#1c2850",
      "--text-primary": "#c0c8e0", "--text-secondary": "#5868a0", "--accent-blue": "#1e3a8a", "--accent-green": "#38b2ac",
      "--accent-purple": "#5b6abf", "--accent-gold": "#c0a030", "--accent-rose": "#c84858",
      "--border-color": "rgba(59, 130, 246, 0.10)", "--error-color": "#c84858",
    },
  },
  {
    id: "light-indigo", name: "Indigo", category: "standard", mode: "light", pairId: "indigo",
    preview: { bg: "#f0f4fc", fg: "#0a1028", accent: "#1e40af", secondary: "#dce4f8" },
    vars: {
      "--bg-primary": "#f0f4fc", "--bg-secondary": "#dce4f8", "--bg-tertiary": "#c4d0f0", "--bg-elevated": "#ffffff",
      "--text-primary": "#0a1028", "--text-secondary": "#4a5888", "--accent-blue": "#1e40af", "--accent-green": "#1a8a80",
      "--accent-purple": "#3a4a98", "--accent-gold": "#987a10", "--accent-rose": "#a03848",
      "--border-color": "rgba(30, 64, 175, 0.10)", "--error-color": "#a03848",
    },
  },
  // ── Pair: Azure (was Maserati) ──
  {
    id: "dark-azure", name: "Azure", category: "standard", mode: "dark", pairId: "azure",
    preview: { bg: "#080c18", fg: "#d0d4e8", accent: "#4a6fa5", secondary: "#101828" },
    vars: {
      "--bg-primary": "#080c18", "--bg-secondary": "#101828", "--bg-tertiary": "#182438", "--bg-elevated": "#203048",
      "--text-primary": "#d0d4e8", "--text-secondary": "#6878a0", "--accent-blue": "#4a6fa5", "--accent-green": "#50a878",
      "--accent-purple": "#7888b8", "--accent-gold": "#b8a050", "--accent-rose": "#b86068",
      "--border-color": "rgba(74, 111, 165, 0.10)", "--error-color": "#b86068",
    },
  },
  {
    id: "light-azure", name: "Azure", category: "standard", mode: "light", pairId: "azure",
    preview: { bg: "#f4f6fb", fg: "#0c1020", accent: "#2c5282", secondary: "#e0e6f2" },
    vars: {
      "--bg-primary": "#f4f6fb", "--bg-secondary": "#e0e6f2", "--bg-tertiary": "#ccd4e6", "--bg-elevated": "#ffffff",
      "--text-primary": "#0c1020", "--text-secondary": "#4a5a80", "--accent-blue": "#2c5282", "--accent-green": "#2a8060",
      "--accent-purple": "#4a6098", "--accent-gold": "#8a7820", "--accent-rose": "#984050",
      "--border-color": "rgba(44, 82, 130, 0.10)", "--error-color": "#984050",
    },
  },
  // ── Restored pairs: the 3 MacBook-inspired palettes removed in the
  // brand-name purge — same colors, renamed with no reference to laptops
  // or phones (category "standard", plain ids).

  // ── Pair: Obsidian (was Space Black) ──
  {
    id: "dark-obsidian", name: "Obsidian", category: "standard", mode: "dark", pairId: "obsidian",
    preview: { bg: "#0c0c0e", fg: "#d8d8dc", accent: "#6eaadc", secondary: "#18181c" },
    vars: {
      "--bg-primary": "#0c0c0e", "--bg-secondary": "#18181c", "--bg-tertiary": "#222228", "--bg-elevated": "#2c2c34",
      "--text-primary": "#d8d8dc", "--text-secondary": "#7a7a86", "--accent-blue": "#6eaadc", "--accent-green": "#5ec27a",
      "--accent-purple": "#b48cda", "--accent-gold": "#e2b84a", "--accent-rose": "#e86070",
      "--border-color": "rgba(110, 170, 220, 0.08)", "--error-color": "#e86070",
    },
  },
  {
    id: "light-obsidian", name: "Obsidian", category: "standard", mode: "light", pairId: "obsidian",
    preview: { bg: "#f4f4f6", fg: "#1c1c22", accent: "#3478f6", secondary: "#e8e8ec" },
    vars: {
      "--bg-primary": "#f4f4f6", "--bg-secondary": "#e8e8ec", "--bg-tertiary": "#d8d8de", "--bg-elevated": "#ffffff",
      "--text-primary": "#1c1c22", "--text-secondary": "#636370", "--accent-blue": "#3478f6", "--accent-green": "#30a856",
      "--accent-purple": "#8944da", "--accent-gold": "#c08a10", "--accent-rose": "#d63852",
      "--border-color": "rgba(52, 120, 246, 0.08)", "--error-color": "#d63852",
    },
  },
  // ── Pair: Nova (was Starlight) ──
  {
    id: "dark-nova", name: "Nova", category: "standard", mode: "dark", pairId: "nova",
    preview: { bg: "#0a0c14", fg: "#d0d4e0", accent: "#4a78c0", secondary: "#141828" },
    vars: {
      "--bg-primary": "#0a0c14", "--bg-secondary": "#141828", "--bg-tertiary": "#1c2238", "--bg-elevated": "#262e48",
      "--text-primary": "#d0d4e0", "--text-secondary": "#6a72a8", "--accent-blue": "#4a78c0", "--accent-green": "#48a870",
      "--accent-purple": "#9a7cc8", "--accent-gold": "#d4aa3a", "--accent-rose": "#d85868",
      "--border-color": "rgba(74, 120, 192, 0.10)", "--error-color": "#d85868",
    },
  },
  {
    id: "light-nova", name: "Nova", category: "standard", mode: "light", pairId: "nova",
    preview: { bg: "#f8f4ee", fg: "#2a2620", accent: "#a0782a", secondary: "#eee8de" },
    vars: {
      "--bg-primary": "#f8f4ee", "--bg-secondary": "#eee8de", "--bg-tertiary": "#e0d8ca", "--bg-elevated": "#fefaf4",
      "--text-primary": "#2a2620", "--text-secondary": "#706452", "--accent-blue": "#a0782a", "--accent-green": "#5a8a40",
      "--accent-purple": "#8a6a98", "--accent-gold": "#b08a18", "--accent-rose": "#b84838",
      "--border-color": "rgba(160, 120, 42, 0.10)", "--error-color": "#b84838",
    },
  },
  // ── Pair: Pewter (was Space Gray) ──
  {
    id: "dark-pewter", name: "Pewter", category: "standard", mode: "dark", pairId: "pewter",
    preview: { bg: "#111113", fg: "#d4d4d8", accent: "#8c8ca0", secondary: "#1c1c20" },
    vars: {
      "--bg-primary": "#111113", "--bg-secondary": "#1c1c20", "--bg-tertiary": "#26262c", "--bg-elevated": "#303038",
      "--text-primary": "#d4d4d8", "--text-secondary": "#78788a", "--accent-blue": "#8c8ca0", "--accent-green": "#5ab872",
      "--accent-purple": "#a888c0", "--accent-gold": "#d0a840", "--accent-rose": "#d86068",
      "--border-color": "rgba(140, 140, 160, 0.10)", "--error-color": "#d86068",
    },
  },
  {
    id: "light-pewter", name: "Pewter", category: "standard", mode: "light", pairId: "pewter",
    preview: { bg: "#f6f6f8", fg: "#1e1e24", accent: "#5a5a72", secondary: "#eaeaee" },
    vars: {
      "--bg-primary": "#f6f6f8", "--bg-secondary": "#eaeaee", "--bg-tertiary": "#dcdce2", "--bg-elevated": "#ffffff",
      "--text-primary": "#1e1e24", "--text-secondary": "#5e5e70", "--accent-blue": "#5a5a72", "--accent-green": "#3a8a4e",
      "--accent-purple": "#6e5a88", "--accent-gold": "#9a8018", "--accent-rose": "#b84048",
      "--border-color": "rgba(90, 90, 114, 0.08)", "--error-color": "#b84048",
    },
  },
  // ── Restored/new pairs: full color-of-the-year archive gap-fill ──
  // Ground-truth swatch colors sampled directly from the reference
  // screenshots (/Volumes/fast01/Colors) rather than estimated; dark/light
  // pairs generated from each swatch's hue via a consistent HSL recipe
  // (background tinted from the hue, accent slots at fixed target hues),
  // matching the treatment used throughout this registry. Names kept as
  // seen, brand attribution dropped per the no-brand-names rule.

  // ── Pair: Renew Blue ──
  {
    id: "dark-renew-blue", name: "Renew Blue", category: "standard", mode: "dark", pairId: "renew-blue",
    preview: { bg: "#111313", fg: "#e5e6e6", accent: "#8cccc8", secondary: "#1a1e1e" },
    vars: {
      "--bg-primary": "#111313", "--bg-secondary": "#1a1e1e", "--bg-tertiary": "#262b2b", "--bg-elevated": "#323938",
      "--text-primary": "#e5e6e6", "--text-secondary": "#909897", "--accent-blue": "#8cccc8", "--accent-green": "#8ccca7",
      "--accent-purple": "#b78ccc", "--accent-gold": "#ccb98c", "--accent-rose": "#cc8c97", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-renew-blue", name: "Renew Blue", category: "standard", mode: "light", pairId: "renew-blue",
    preview: { bg: "#f7f8f8", fg: "#222525", accent: "#469d97", secondary: "#eceeee" },
    vars: {
      "--bg-primary": "#f7f8f8", "--bg-secondary": "#eceeee", "--bg-tertiary": "#dddfdf", "--bg-elevated": "#ffffff",
      "--text-primary": "#222525", "--text-secondary": "#636969", "--accent-blue": "#469d97", "--accent-green": "#469d6a",
      "--accent-purple": "#80469d", "--accent-gold": "#9d8346", "--accent-rose": "#9d4655", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Encore ──
  {
    id: "dark-encore", name: "Encore", category: "standard", mode: "dark", pairId: "encore",
    preview: { bg: "#0f1115", fg: "#e3e5e8", accent: "#4c69a9", secondary: "#171a21" },
    vars: {
      "--bg-primary": "#0f1115", "--bg-secondary": "#171a21", "--bg-tertiary": "#22262f", "--bg-elevated": "#2d323e",
      "--text-primary": "#e3e5e8", "--text-secondary": "#89909f", "--accent-blue": "#4c69a9", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-encore", name: "Encore", category: "standard", mode: "light", pairId: "encore",
    preview: { bg: "#f7f7f8", fg: "#202227", accent: "#415a90", secondary: "#ebecef" },
    vars: {
      "--bg-primary": "#f7f7f8", "--bg-secondary": "#ebecef", "--bg-tertiary": "#dadde1", "--bg-elevated": "#ffffff",
      "--text-primary": "#202227", "--text-secondary": "#5d636f", "--accent-blue": "#415a90", "--accent-green": "#419062",
      "--accent-purple": "#764190", "--accent-gold": "#907841", "--accent-rose": "#90414e", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Midnight Garden ──
  {
    id: "dark-midnight-garden", name: "Midnight Garden", category: "standard", mode: "dark", pairId: "midnight-garden",
    preview: { bg: "#121311", fg: "#e6e6e5", accent: "#87a94c", secondary: "#1c1e1a" },
    vars: {
      "--bg-primary": "#121311", "--bg-secondary": "#1c1e1a", "--bg-tertiary": "#292b26", "--bg-elevated": "#363932",
      "--text-primary": "#e6e6e5", "--text-secondary": "#959790", "--accent-blue": "#87a94c", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-midnight-garden", name: "Midnight Garden", category: "standard", mode: "light", pairId: "midnight-garden",
    preview: { bg: "#f7f8f7", fg: "#242523", accent: "#87a94c", secondary: "#edeeec" },
    vars: {
      "--bg-primary": "#f7f8f7", "--bg-secondary": "#edeeec", "--bg-tertiary": "#dedfdd", "--bg-elevated": "#ffffff",
      "--text-primary": "#242523", "--text-secondary": "#676963", "--accent-blue": "#87a94c", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Caramelized ──
  {
    id: "dark-caramelized", name: "Caramelized", category: "standard", mode: "dark", pairId: "caramelized",
    preview: { bg: "#141110", fg: "#e7e5e4", accent: "#c19276", secondary: "#201b19" },
    vars: {
      "--bg-primary": "#141110", "--bg-secondary": "#201b19", "--bg-tertiary": "#2e2824", "--bg-elevated": "#3c342f",
      "--text-primary": "#e7e5e4", "--text-secondary": "#9c928b", "--accent-blue": "#c19276", "--accent-green": "#76c195",
      "--accent-purple": "#a876c1", "--accent-gold": "#c1ab76", "--accent-rose": "#c17682", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-caramelized", name: "Caramelized", category: "standard", mode: "light", pairId: "caramelized",
    preview: { bg: "#f8f7f7", fg: "#272321", accent: "#a96f4c", secondary: "#efedec" },
    vars: {
      "--bg-primary": "#f8f7f7", "--bg-secondary": "#efedec", "--bg-tertiary": "#e0dddb", "--bg-elevated": "#ffffff",
      "--text-primary": "#272321", "--text-secondary": "#6d645f", "--accent-blue": "#a96f4c", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Skipping Stones ──
  {
    id: "dark-skipping-stones", name: "Skipping Stones", category: "standard", mode: "dark", pairId: "skipping-stones",
    preview: { bg: "#101213", fg: "#e4e6e7", accent: "#72a8c0", secondary: "#1a1d1e" },
    vars: {
      "--bg-primary": "#101213", "--bg-secondary": "#1a1d1e", "--bg-tertiary": "#252a2c", "--bg-elevated": "#31373a",
      "--text-primary": "#e4e6e7", "--text-secondary": "#8e969a", "--accent-blue": "#72a8c0", "--accent-green": "#72c093",
      "--accent-purple": "#a672c0", "--accent-gold": "#c0a972", "--accent-rose": "#c0727f", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-skipping-stones", name: "Skipping Stones", category: "standard", mode: "light", pairId: "skipping-stones",
    preview: { bg: "#f7f8f8", fg: "#222426", accent: "#4c8ca9", secondary: "#eceeee" },
    vars: {
      "--bg-primary": "#f7f8f8", "--bg-secondary": "#eceeee", "--bg-tertiary": "#dcdfe0", "--bg-elevated": "#ffffff",
      "--text-primary": "#222426", "--text-secondary": "#61686b", "--accent-blue": "#4c8ca9", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Terra Rosa ──
  {
    id: "dark-terra-rosa", name: "Terra Rosa", category: "standard", mode: "dark", pairId: "terra-rosa",
    preview: { bg: "#140f0f", fg: "#e7e4e4", accent: "#b65d5d", secondary: "#201818" },
    vars: {
      "--bg-primary": "#140f0f", "--bg-secondary": "#201818", "--bg-tertiary": "#2e2323", "--bg-elevated": "#3d2e2e",
      "--text-primary": "#e7e4e4", "--text-secondary": "#9d8b8b", "--accent-blue": "#b65d5d", "--accent-green": "#5db682",
      "--accent-purple": "#985db6", "--accent-gold": "#b69b5d", "--accent-rose": "#b65d6c", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-terra-rosa", name: "Terra Rosa", category: "standard", mode: "light", pairId: "terra-rosa",
    preview: { bg: "#f8f7f7", fg: "#272121", accent: "#a94c4c", secondary: "#efecec" },
    vars: {
      "--bg-primary": "#f8f7f7", "--bg-secondary": "#efecec", "--bg-tertiary": "#e1dbdb", "--bg-elevated": "#ffffff",
      "--text-primary": "#272121", "--text-secondary": "#6d5f5f", "--accent-blue": "#a94c4c", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Art and Craft ──
  {
    id: "dark-art-and-craft", name: "Art and Craft", category: "standard", mode: "dark", pairId: "art-and-craft",
    preview: { bg: "#131211", fg: "#e7e5e4", accent: "#a9714c", secondary: "#1e1c1a" },
    vars: {
      "--bg-primary": "#131211", "--bg-secondary": "#1e1c1a", "--bg-tertiary": "#2c2826", "--bg-elevated": "#393532",
      "--text-primary": "#e7e5e4", "--text-secondary": "#99938f", "--accent-blue": "#a9714c", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-art-and-craft", name: "Art and Craft", category: "standard", mode: "light", pairId: "art-and-craft",
    preview: { bg: "#f8f7f7", fg: "#252322", accent: "#a9714c", secondary: "#eeedec" },
    vars: {
      "--bg-primary": "#f8f7f7", "--bg-secondary": "#eeedec", "--bg-tertiary": "#dfdedc", "--bg-elevated": "#ffffff",
      "--text-primary": "#252322", "--text-secondary": "#6a6562", "--accent-blue": "#a9714c", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Wild Blue Yonder ──
  {
    id: "dark-wild-blue-yonder", name: "Wild Blue Yonder", category: "standard", mode: "dark", pairId: "wild-blue-yonder",
    preview: { bg: "#0c1317", fg: "#e1e6ea", accent: "#78b5e3", secondary: "#141d24" },
    vars: {
      "--bg-primary": "#0c1317", "--bg-secondary": "#141d24", "--bg-tertiary": "#1d2b35", "--bg-elevated": "#253846",
      "--text-primary": "#e1e6ea", "--text-secondary": "#7b97ac", "--accent-blue": "#78b5e3", "--accent-green": "#78e3a4",
      "--accent-purple": "#bf78e3", "--accent-gold": "#e3c378", "--accent-rose": "#e3788a", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-wild-blue-yonder", name: "Wild Blue Yonder", category: "standard", mode: "light", pairId: "wild-blue-yonder",
    preview: { bg: "#f6f8f9", fg: "#1c252c", accent: "#1c5987", secondary: "#eaeef0" },
    vars: {
      "--bg-primary": "#f6f8f9", "--bg-secondary": "#eaeef0", "--bg-tertiary": "#d8dfe4", "--bg-elevated": "#ffffff",
      "--text-primary": "#1c252c", "--text-secondary": "#52697a", "--accent-blue": "#1c5987", "--accent-green": "#1c8749",
      "--accent-purple": "#631c87", "--accent-gold": "#87671c", "--accent-rose": "#871c2e", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Minty Fresh ──
  {
    id: "dark-minty-fresh", name: "Minty Fresh", category: "standard", mode: "dark", pairId: "minty-fresh",
    preview: { bg: "#0f1512", fg: "#e3e8e5", accent: "#8eccac", secondary: "#18201c" },
    vars: {
      "--bg-primary": "#0f1512", "--bg-secondary": "#18201c", "--bg-tertiary": "#222f28", "--bg-elevated": "#2d3e35",
      "--text-primary": "#e3e8e5", "--text-secondary": "#899f93", "--accent-blue": "#8eccac", "--accent-green": "#8ecca8",
      "--accent-purple": "#b88ecc", "--accent-gold": "#ccba8e", "--accent-rose": "#cc8e99", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-minty-fresh", name: "Minty Fresh", category: "standard", mode: "light", pairId: "minty-fresh",
    preview: { bg: "#f7f8f7", fg: "#202723", accent: "#337150", secondary: "#ebefed" },
    vars: {
      "--bg-primary": "#f7f8f7", "--bg-secondary": "#ebefed", "--bg-tertiary": "#dbe1de", "--bg-elevated": "#ffffff",
      "--text-primary": "#202723", "--text-secondary": "#5d6f65", "--accent-blue": "#337150", "--accent-green": "#33714c",
      "--accent-purple": "#5c3371", "--accent-gold": "#715e33", "--accent-rose": "#71333d", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Honey Glow ──
  {
    id: "dark-honey-glow", name: "Honey Glow", category: "standard", mode: "dark", pairId: "honey-glow",
    preview: { bg: "#17140c", fg: "#eae7e1", accent: "#e3be6d", secondary: "#241f14" },
    vars: {
      "--bg-primary": "#17140c", "--bg-secondary": "#241f14", "--bg-tertiary": "#352d1d", "--bg-elevated": "#463c25",
      "--text-primary": "#eae7e1", "--text-secondary": "#ad9d7a", "--accent-blue": "#e3be6d", "--accent-green": "#6de39e",
      "--accent-purple": "#bc6de3", "--accent-gold": "#e3c06d", "--accent-rose": "#e36d81", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-honey-glow", name: "Honey Glow", category: "standard", mode: "light", pairId: "honey-glow",
    preview: { bg: "#f9f8f6", fg: "#2c271c", accent: "#c59326", secondary: "#f0eeea" },
    vars: {
      "--bg-primary": "#f9f8f6", "--bg-secondary": "#f0eeea", "--bg-tertiary": "#e4e0d8", "--bg-elevated": "#ffffff",
      "--text-primary": "#2c271c", "--text-secondary": "#7a6e52", "--accent-blue": "#c59326", "--accent-green": "#26c568",
      "--accent-purple": "#9026c5", "--accent-gold": "#c59626", "--accent-rose": "#c52640", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: The Green Hour ──
  {
    id: "dark-the-green-hour", name: "The Green Hour", category: "standard", mode: "dark", pairId: "the-green-hour",
    preview: { bg: "#101313", fg: "#e4e7e7", accent: "#6fbeb7", secondary: "#191f1e" },
    vars: {
      "--bg-primary": "#101313", "--bg-secondary": "#191f1e", "--bg-tertiary": "#252d2c", "--bg-elevated": "#313a3a",
      "--text-primary": "#e4e7e7", "--text-secondary": "#8e9a99", "--accent-blue": "#6fbeb7", "--accent-green": "#6fbe90",
      "--accent-purple": "#a46fbe", "--accent-gold": "#bea66f", "--accent-rose": "#be6f7c", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-the-green-hour", name: "The Green Hour", category: "standard", mode: "light", pairId: "the-green-hour",
    preview: { bg: "#f7f8f8", fg: "#222625", accent: "#4ca9a1", secondary: "#eceeee" },
    vars: {
      "--bg-primary": "#f7f8f8", "--bg-secondary": "#eceeee", "--bg-tertiary": "#dce0df", "--bg-elevated": "#ffffff",
      "--text-primary": "#222625", "--text-secondary": "#616b6a", "--accent-blue": "#4ca9a1", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Spice of Life ──
  {
    id: "dark-spice-of-life", name: "Spice of Life", category: "standard", mode: "dark", pairId: "spice-of-life",
    preview: { bg: "#14100f", fg: "#e8e4e3", accent: "#a95d4c", secondary: "#201918" },
    vars: {
      "--bg-primary": "#14100f", "--bg-secondary": "#201918", "--bg-tertiary": "#2f2523", "--bg-elevated": "#3d312e",
      "--text-primary": "#e8e4e3", "--text-secondary": "#9e8e8a", "--accent-blue": "#a95d4c", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-spice-of-life", name: "Spice of Life", category: "standard", mode: "light", pairId: "spice-of-life",
    preview: { bg: "#f8f7f7", fg: "#272220", accent: "#a65c4b", secondary: "#efeceb" },
    vars: {
      "--bg-primary": "#f8f7f7", "--bg-secondary": "#efeceb", "--bg-tertiary": "#e1dcdb", "--bg-elevated": "#ffffff",
      "--text-primary": "#272220", "--text-secondary": "#6e615e", "--accent-blue": "#a65c4b", "--accent-green": "#4ba671",
      "--accent-purple": "#884ba6", "--accent-gold": "#a68b4b", "--accent-rose": "#a64b5a", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Melodious Ivory ──
  {
    id: "dark-melodious-ivory", name: "Melodious Ivory", category: "standard", mode: "dark", pairId: "melodious-ivory",
    preview: { bg: "#16130d", fg: "#e9e7e2", accent: "#d3b988", secondary: "#231e15" },
    vars: {
      "--bg-primary": "#16130d", "--bg-secondary": "#231e15", "--bg-tertiary": "#332c1f", "--bg-elevated": "#433a28",
      "--text-primary": "#e9e7e2", "--text-secondary": "#a59983", "--accent-blue": "#d3b988", "--accent-green": "#88d3a7",
      "--accent-purple": "#ba88d3", "--accent-gold": "#d3bc88", "--accent-rose": "#d38894", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-melodious-ivory", name: "Melodious Ivory", category: "standard", mode: "light", pairId: "melodious-ivory",
    preview: { bg: "#f9f8f6", fg: "#29261e", accent: "#775e2c", secondary: "#f0eeea" },
    vars: {
      "--bg-primary": "#f9f8f6", "--bg-secondary": "#f0eeea", "--bg-tertiary": "#e3e0d9", "--bg-elevated": "#ffffff",
      "--text-primary": "#29261e", "--text-secondary": "#746a58", "--accent-blue": "#775e2c", "--accent-green": "#2c774b",
      "--accent-purple": "#5e2c77", "--accent-gold": "#77612c", "--accent-rose": "#772c39", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Mapped Blue ──
  {
    id: "dark-mapped-blue", name: "Mapped Blue", category: "standard", mode: "dark", pairId: "mapped-blue",
    preview: { bg: "#111213", fg: "#e4e6e7", accent: "#64a3ba", secondary: "#1a1d1e" },
    vars: {
      "--bg-primary": "#111213", "--bg-secondary": "#1a1d1e", "--bg-tertiary": "#262a2c", "--bg-elevated": "#323739",
      "--text-primary": "#e4e6e7", "--text-secondary": "#8f9699", "--accent-blue": "#64a3ba", "--accent-green": "#64ba88",
      "--accent-purple": "#9d64ba", "--accent-gold": "#baa064", "--accent-rose": "#ba6473", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-mapped-blue", name: "Mapped Blue", category: "standard", mode: "light", pairId: "mapped-blue",
    preview: { bg: "#f7f8f8", fg: "#222425", accent: "#4c90a9", secondary: "#eceeee" },
    vars: {
      "--bg-primary": "#f7f8f8", "--bg-secondary": "#eceeee", "--bg-tertiary": "#dcdfdf", "--bg-elevated": "#ffffff",
      "--text-primary": "#222425", "--text-secondary": "#62686a", "--accent-blue": "#4c90a9", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Ironside ──
  {
    id: "dark-ironside", name: "Ironside", category: "standard", mode: "dark", pairId: "ironside",
    preview: { bg: "#131311", fg: "#e6e6e5", accent: "#a9a04c", secondary: "#1e1d1a" },
    vars: {
      "--bg-primary": "#131311", "--bg-secondary": "#1e1d1a", "--bg-tertiary": "#2b2b26", "--bg-elevated": "#393832",
      "--text-primary": "#e6e6e5", "--text-secondary": "#979691", "--accent-blue": "#a9a04c", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-ironside", name: "Ironside", category: "standard", mode: "light", pairId: "ironside",
    preview: { bg: "#f8f8f7", fg: "#252523", accent: "#89823d", secondary: "#eeeeec" },
    vars: {
      "--bg-primary": "#f8f8f7", "--bg-secondary": "#eeeeec", "--bg-tertiary": "#dfdfdd", "--bg-elevated": "#ffffff",
      "--text-primary": "#252523", "--text-secondary": "#686864", "--accent-blue": "#89823d", "--accent-green": "#3d895d",
      "--accent-purple": "#703d89", "--accent-gold": "#89723d", "--accent-rose": "#893d4a", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Rustic Greige ──
  {
    id: "dark-rustic-greige", name: "Rustic Greige", category: "standard", mode: "dark", pairId: "rustic-greige",
    preview: { bg: "#131211", fg: "#e6e5e5", accent: "#ba8d65", secondary: "#1e1c1a" },
    vars: {
      "--bg-primary": "#131211", "--bg-secondary": "#1e1c1a", "--bg-tertiary": "#2b2926", "--bg-elevated": "#393532",
      "--text-primary": "#e6e5e5", "--text-secondary": "#979491", "--accent-blue": "#ba8d65", "--accent-green": "#65ba88",
      "--accent-purple": "#9e65ba", "--accent-gold": "#baa065", "--accent-rose": "#ba6573", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-rustic-greige", name: "Rustic Greige", category: "standard", mode: "light", pairId: "rustic-greige",
    preview: { bg: "#f8f7f7", fg: "#252423", accent: "#a9784c", secondary: "#eeedec" },
    vars: {
      "--bg-primary": "#f8f7f7", "--bg-secondary": "#eeedec", "--bg-tertiary": "#dfdedd", "--bg-elevated": "#ffffff",
      "--text-primary": "#252423", "--text-secondary": "#696663", "--accent-blue": "#a9784c", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Cypress Garden ──
  {
    id: "dark-cypress-garden", name: "Cypress Garden", category: "standard", mode: "dark", pairId: "cypress-garden",
    preview: { bg: "#121311", fg: "#e6e6e5", accent: "#a5bd6d", secondary: "#1d1e1a" },
    vars: {
      "--bg-primary": "#121311", "--bg-secondary": "#1d1e1a", "--bg-tertiary": "#2a2b26", "--bg-elevated": "#373932",
      "--text-primary": "#e6e6e5", "--text-secondary": "#959692", "--accent-blue": "#a5bd6d", "--accent-green": "#6dbd8e",
      "--accent-purple": "#a26dbd", "--accent-gold": "#bda56d", "--accent-rose": "#bd6d7a", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-cypress-garden", name: "Cypress Garden", category: "standard", mode: "light", pairId: "cypress-garden",
    preview: { bg: "#f7f8f7", fg: "#242423", accent: "#8da94c", secondary: "#edeeec" },
    vars: {
      "--bg-primary": "#f7f8f7", "--bg-secondary": "#edeeec", "--bg-tertiary": "#dedfdd", "--bg-elevated": "#ffffff",
      "--text-primary": "#242423", "--text-secondary": "#676765", "--accent-blue": "#8da94c", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Earth's Harmony ──
  {
    id: "dark-earths-harmony", name: "Earth's Harmony", category: "standard", mode: "dark", pairId: "earths-harmony",
    preview: { bg: "#0e1215", fg: "#e3e6e8", accent: "#8eb0cc", secondary: "#171c22" },
    vars: {
      "--bg-primary": "#0e1215", "--bg-secondary": "#171c22", "--bg-tertiary": "#212931", "--bg-elevated": "#2b3640",
      "--text-primary": "#e3e6e8", "--text-secondary": "#8795a1", "--accent-blue": "#8eb0cc", "--accent-green": "#8ecca8",
      "--accent-purple": "#b88ecc", "--accent-gold": "#ccba8e", "--accent-rose": "#cc8e99", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-earths-harmony", name: "Earth's Harmony", category: "standard", mode: "light", pairId: "earths-harmony",
    preview: { bg: "#f6f7f8", fg: "#1f2428", accent: "#3e688b", secondary: "#ebedef" },
    vars: {
      "--bg-primary": "#f6f7f8", "--bg-secondary": "#ebedef", "--bg-tertiary": "#dadee2", "--bg-elevated": "#ffffff",
      "--text-primary": "#1f2428", "--text-secondary": "#5b6771", "--accent-blue": "#3e688b", "--accent-green": "#3e8b5e",
      "--accent-purple": "#713e8b", "--accent-gold": "#8b743e", "--accent-rose": "#8b3e4b", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Jasmine Flower ──
  {
    id: "dark-jasmine-flower", name: "Jasmine Flower", category: "standard", mode: "dark", pairId: "jasmine-flower",
    preview: { bg: "#131112", fg: "#e6e5e5", accent: "#c57d9b", secondary: "#1e1a1c" },
    vars: {
      "--bg-primary": "#131112", "--bg-secondary": "#1e1a1c", "--bg-tertiary": "#2c2628", "--bg-elevated": "#393235",
      "--text-primary": "#e6e5e5", "--text-secondary": "#998f93", "--accent-blue": "#c57d9b", "--accent-green": "#7dc59b",
      "--accent-purple": "#ad7dc5", "--accent-gold": "#c5af7d", "--accent-rose": "#c57d89", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-jasmine-flower", name: "Jasmine Flower", category: "standard", mode: "light", pairId: "jasmine-flower",
    preview: { bg: "#f8f7f7", fg: "#252223", accent: "#a94c73", secondary: "#eeeced" },
    vars: {
      "--bg-primary": "#f8f7f7", "--bg-secondary": "#eeeced", "--bg-tertiary": "#dfdcde", "--bg-elevated": "#ffffff",
      "--text-primary": "#252223", "--text-secondary": "#6a6265", "--accent-blue": "#a94c73", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Garden Patch ──
  {
    id: "dark-garden-patch", name: "Garden Patch", category: "standard", mode: "dark", pairId: "garden-patch",
    preview: { bg: "#121311", fg: "#e6e6e5", accent: "#8cb150", secondary: "#1c1e1a" },
    vars: {
      "--bg-primary": "#121311", "--bg-secondary": "#1c1e1a", "--bg-tertiary": "#292b26", "--bg-elevated": "#363932",
      "--text-primary": "#e6e6e5", "--text-secondary": "#959791", "--accent-blue": "#8cb150", "--accent-green": "#50b178",
      "--accent-purple": "#9150b1", "--accent-gold": "#b19450", "--accent-rose": "#b15060", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-garden-patch", name: "Garden Patch", category: "standard", mode: "light", pairId: "garden-patch",
    preview: { bg: "#f7f8f7", fg: "#242523", accent: "#85a94c", secondary: "#edeeec" },
    vars: {
      "--bg-primary": "#f7f8f7", "--bg-secondary": "#edeeec", "--bg-tertiary": "#dedfdd", "--bg-elevated": "#ffffff",
      "--text-primary": "#242523", "--text-secondary": "#676963", "--accent-blue": "#85a94c", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Sandstone Tint ──
  {
    id: "dark-sandstone-tint", name: "Sandstone Tint", category: "standard", mode: "dark", pairId: "sandstone-tint",
    preview: { bg: "#131210", fg: "#e7e6e4", accent: "#ccb68e", secondary: "#1f1d1a" },
    vars: {
      "--bg-primary": "#131210", "--bg-secondary": "#1f1d1a", "--bg-tertiary": "#2c2a25", "--bg-elevated": "#3a3731",
      "--text-primary": "#e7e6e4", "--text-secondary": "#9a958e", "--accent-blue": "#ccb68e", "--accent-green": "#8ecca8",
      "--accent-purple": "#b88ecc", "--accent-gold": "#ccba8e", "--accent-rose": "#cc8e99", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-sandstone-tint", name: "Sandstone Tint", category: "standard", mode: "light", pairId: "sandstone-tint",
    preview: { bg: "#f8f7f7", fg: "#262422", accent: "#7c6338", secondary: "#eeedec" },
    vars: {
      "--bg-primary": "#f8f7f7", "--bg-secondary": "#eeedec", "--bg-tertiary": "#e0dedc", "--bg-elevated": "#ffffff",
      "--text-primary": "#262422", "--text-secondary": "#6b6761", "--accent-blue": "#7c6338", "--accent-green": "#387c54",
      "--accent-purple": "#66387c", "--accent-gold": "#7c6838", "--accent-rose": "#7c3843", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: In the Brush ──
  {
    id: "dark-in-the-brush", name: "In the Brush", category: "standard", mode: "dark", pairId: "in-the-brush",
    preview: { bg: "#111311", fg: "#e5e6e5", accent: "#4ca963", secondary: "#1a1e1b" },
    vars: {
      "--bg-primary": "#111311", "--bg-secondary": "#1a1e1b", "--bg-tertiary": "#262b28", "--bg-elevated": "#323934",
      "--text-primary": "#e5e6e5", "--text-secondary": "#939593", "--accent-blue": "#4ca963", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-in-the-brush", name: "In the Brush", category: "standard", mode: "light", pairId: "in-the-brush",
    preview: { bg: "#f7f8f7", fg: "#232423", accent: "#4ca963", secondary: "#eceeed" },
    vars: {
      "--bg-primary": "#f7f8f7", "--bg-secondary": "#eceeed", "--bg-tertiary": "#dddfdd", "--bg-elevated": "#ffffff",
      "--text-primary": "#232423", "--text-secondary": "#656765", "--accent-blue": "#4ca963", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Coffee Bean ──
  {
    id: "dark-coffee-bean", name: "Coffee Bean", category: "standard", mode: "dark", pairId: "coffee-bean",
    preview: { bg: "#131211", fg: "#e6e5e5", accent: "#a9774c", secondary: "#1e1c1a" },
    vars: {
      "--bg-primary": "#131211", "--bg-secondary": "#1e1c1a", "--bg-tertiary": "#2b2926", "--bg-elevated": "#393532",
      "--text-primary": "#e6e5e5", "--text-secondary": "#989490", "--accent-blue": "#a9774c", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-coffee-bean", name: "Coffee Bean", category: "standard", mode: "light", pairId: "coffee-bean",
    preview: { bg: "#f8f7f7", fg: "#252422", accent: "#855d3c", secondary: "#eeedec" },
    vars: {
      "--bg-primary": "#f8f7f7", "--bg-secondary": "#eeedec", "--bg-tertiary": "#dfdedd", "--bg-elevated": "#ffffff",
      "--text-primary": "#252422", "--text-secondary": "#696663", "--accent-blue": "#855d3c", "--accent-green": "#3c855a",
      "--accent-purple": "#6c3c85", "--accent-gold": "#856f3c", "--accent-rose": "#853c48", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Hammered Black ──
  {
    id: "dark-hammered-black", name: "Hammered Black", category: "standard", mode: "dark", pairId: "hammered-black",
    preview: { bg: "#111213", fg: "#e5e6e6", accent: "#4c7aa9", secondary: "#1a1c1e" },
    vars: {
      "--bg-primary": "#111213", "--bg-secondary": "#1a1c1e", "--bg-tertiary": "#26292c", "--bg-elevated": "#323639",
      "--text-primary": "#e5e6e6", "--text-secondary": "#8f9499", "--accent-blue": "#4c7aa9", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-hammered-black", name: "Hammered Black", category: "standard", mode: "light", pairId: "hammered-black",
    preview: { bg: "#f7f7f8", fg: "#222425", accent: "#335271", secondary: "#ecedee" },
    vars: {
      "--bg-primary": "#f7f7f8", "--bg-secondary": "#ecedee", "--bg-tertiary": "#dcdedf", "--bg-elevated": "#ffffff",
      "--text-primary": "#222425", "--text-secondary": "#62666a", "--accent-blue": "#335271", "--accent-green": "#33714c",
      "--accent-purple": "#5c3371", "--accent-gold": "#715e33", "--accent-rose": "#71333d", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Bluebird ──
  {
    id: "dark-bluebird", name: "Bluebird", category: "standard", mode: "dark", pairId: "bluebird",
    preview: { bg: "#0d1216", fg: "#e2e6e9", accent: "#5994c2", secondary: "#151d23" },
    vars: {
      "--bg-primary": "#0d1216", "--bg-secondary": "#151d23", "--bg-tertiary": "#1e2a33", "--bg-elevated": "#283743",
      "--text-primary": "#e2e6e9", "--text-secondary": "#8396a5", "--accent-blue": "#5994c2", "--accent-green": "#59c285",
      "--accent-purple": "#9f59c2", "--accent-gold": "#c2a359", "--accent-rose": "#c2596b", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-bluebird", name: "Bluebird", category: "standard", mode: "light", pairId: "bluebird",
    preview: { bg: "#f6f8f9", fg: "#1e2429", accent: "#4281b3", secondary: "#eaeef0" },
    vars: {
      "--bg-primary": "#f6f8f9", "--bg-secondary": "#eaeef0", "--bg-tertiary": "#d8dfe3", "--bg-elevated": "#ffffff",
      "--text-primary": "#1e2429", "--text-secondary": "#586874", "--accent-blue": "#4281b3", "--accent-green": "#42b371",
      "--accent-purple": "#8d42b3", "--accent-gold": "#b39142", "--accent-rose": "#b34255", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Spanish Moss ──
  {
    id: "dark-spanish-moss", name: "Spanish Moss", category: "standard", mode: "dark", pairId: "spanish-moss",
    preview: { bg: "#111311", fg: "#e5e6e5", accent: "#63a94c", secondary: "#1b1e1a" },
    vars: {
      "--bg-primary": "#111311", "--bg-secondary": "#1b1e1a", "--bg-tertiary": "#282b26", "--bg-elevated": "#343932",
      "--text-primary": "#e5e6e5", "--text-secondary": "#929791", "--accent-blue": "#63a94c", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-spanish-moss", name: "Spanish Moss", category: "standard", mode: "light", pairId: "spanish-moss",
    preview: { bg: "#f7f8f7", fg: "#232523", accent: "#518b3e", secondary: "#edeeec" },
    vars: {
      "--bg-primary": "#f7f8f7", "--bg-secondary": "#edeeec", "--bg-tertiary": "#dddfdd", "--bg-elevated": "#ffffff",
      "--text-primary": "#232523", "--text-secondary": "#656963", "--accent-blue": "#518b3e", "--accent-green": "#3e8b5e",
      "--accent-purple": "#713e8b", "--accent-gold": "#8b743e", "--accent-rose": "#8b3e4b", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Rolling Surf ──
  {
    id: "dark-rolling-surf", name: "Rolling Surf", category: "standard", mode: "dark", pairId: "rolling-surf",
    preview: { bg: "#101314", fg: "#e4e6e7", accent: "#4c96a9", secondary: "#191e1f" },
    vars: {
      "--bg-primary": "#101314", "--bg-secondary": "#191e1f", "--bg-tertiary": "#242c2e", "--bg-elevated": "#2f393c",
      "--text-primary": "#e4e6e7", "--text-secondary": "#8c999c", "--accent-blue": "#4c96a9", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-rolling-surf", name: "Rolling Surf", category: "standard", mode: "light", pairId: "rolling-surf",
    preview: { bg: "#f7f8f8", fg: "#212526", accent: "#4c96a9", secondary: "#eceeef" },
    vars: {
      "--bg-primary": "#f7f8f8", "--bg-secondary": "#eceeef", "--bg-tertiary": "#dbdfe0", "--bg-elevated": "#ffffff",
      "--text-primary": "#212526", "--text-secondary": "#5f6a6d", "--accent-blue": "#4c96a9", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Warm Caramel ──
  {
    id: "dark-warm-caramel", name: "Warm Caramel", category: "standard", mode: "dark", pairId: "warm-caramel",
    preview: { bg: "#15110f", fg: "#e8e5e3", accent: "#b17652", secondary: "#211b17" },
    vars: {
      "--bg-primary": "#15110f", "--bg-secondary": "#211b17", "--bg-tertiary": "#302722", "--bg-elevated": "#3f332c",
      "--text-primary": "#e8e5e3", "--text-secondary": "#a09188", "--accent-blue": "#b17652", "--accent-green": "#52b17a",
      "--accent-purple": "#9152b1", "--accent-gold": "#b19552", "--accent-rose": "#b15262", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-warm-caramel", name: "Warm Caramel", category: "standard", mode: "light", pairId: "warm-caramel",
    preview: { bg: "#f8f7f6", fg: "#282320", accent: "#a96f4c", secondary: "#efedeb" },
    vars: {
      "--bg-primary": "#f8f7f6", "--bg-secondary": "#efedeb", "--bg-tertiary": "#e2ddda", "--bg-elevated": "#ffffff",
      "--text-primary": "#282320", "--text-secondary": "#70645c", "--accent-blue": "#a96f4c", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: French Blue ──
  {
    id: "dark-french-blue", name: "French Blue", category: "standard", mode: "dark", pairId: "french-blue",
    preview: { bg: "#0f1215", fg: "#e3e6e8", accent: "#8eb0cc", secondary: "#171c21" },
    vars: {
      "--bg-primary": "#0f1215", "--bg-secondary": "#171c21", "--bg-tertiary": "#222930", "--bg-elevated": "#2d363e",
      "--text-primary": "#e3e6e8", "--text-secondary": "#89959f", "--accent-blue": "#8eb0cc", "--accent-green": "#8ecca8",
      "--accent-purple": "#b88ecc", "--accent-gold": "#ccba8e", "--accent-rose": "#cc8e99", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-french-blue", name: "French Blue", category: "standard", mode: "light", pairId: "french-blue",
    preview: { bg: "#f7f7f8", fg: "#202427", accent: "#406b8f", secondary: "#ebedef" },
    vars: {
      "--bg-primary": "#f7f7f8", "--bg-secondary": "#ebedef", "--bg-tertiary": "#dadee1", "--bg-elevated": "#ffffff",
      "--text-primary": "#202427", "--text-secondary": "#5d676f", "--accent-blue": "#406b8f", "--accent-green": "#408f61",
      "--accent-purple": "#75408f", "--accent-gold": "#8f7740", "--accent-rose": "#8f404d", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Gloss Grape ──
  {
    id: "dark-gloss-grape", name: "Gloss Grape", category: "standard", mode: "dark", pairId: "gloss-grape",
    preview: { bg: "#0f0e16", fg: "#e4e2e9", accent: "#5b49ac", secondary: "#181622" },
    vars: {
      "--bg-primary": "#0f0e16", "--bg-secondary": "#181622", "--bg-tertiary": "#232032", "--bg-elevated": "#2e2a42",
      "--text-primary": "#e4e2e9", "--text-secondary": "#8a85a3", "--accent-blue": "#5b49ac", "--accent-green": "#49ac72",
      "--accent-purple": "#8b49ac", "--accent-gold": "#ac8e49", "--accent-rose": "#ac4959", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-gloss-grape", name: "Gloss Grape", category: "standard", mode: "light", pairId: "gloss-grape",
    preview: { bg: "#f7f6f8", fg: "#201f29", accent: "#5b49ac", secondary: "#ecebf0" },
    vars: {
      "--bg-primary": "#f7f6f8", "--bg-secondary": "#ecebf0", "--bg-tertiary": "#dbd9e3", "--bg-elevated": "#ffffff",
      "--text-primary": "#201f29", "--text-secondary": "#5e5a72", "--accent-blue": "#5b49ac", "--accent-green": "#49ac72",
      "--accent-purple": "#8b49ac", "--accent-gold": "#ac8e49", "--accent-rose": "#ac4959", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Hunt Club Green ──
  {
    id: "dark-hunt-club-green", name: "Hunt Club Green", category: "standard", mode: "dark", pairId: "hunt-club-green",
    preview: { bg: "#111311", fg: "#e5e6e5", accent: "#4ca958", secondary: "#1a1e1b" },
    vars: {
      "--bg-primary": "#111311", "--bg-secondary": "#1a1e1b", "--bg-tertiary": "#262b27", "--bg-elevated": "#323933",
      "--text-primary": "#e5e6e5", "--text-secondary": "#929692", "--accent-blue": "#4ca958", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-hunt-club-green", name: "Hunt Club Green", category: "standard", mode: "light", pairId: "hunt-club-green",
    preview: { bg: "#f7f8f7", fg: "#232423", accent: "#44974e", secondary: "#eceeed" },
    vars: {
      "--bg-primary": "#f7f8f7", "--bg-secondary": "#eceeed", "--bg-tertiary": "#dddfdd", "--bg-elevated": "#ffffff",
      "--text-primary": "#232423", "--text-secondary": "#646865", "--accent-blue": "#44974e", "--accent-green": "#449767",
      "--accent-purple": "#7b4497", "--accent-gold": "#977e44", "--accent-rose": "#974452", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Paprika ──
  {
    id: "dark-paprika", name: "Paprika", category: "standard", mode: "dark", pairId: "paprika",
    preview: { bg: "#170e0d", fg: "#eae2e1", accent: "#bb4a3a", secondary: "#241614" },
    vars: {
      "--bg-primary": "#170e0d", "--bg-secondary": "#241614", "--bg-tertiary": "#35201d", "--bg-elevated": "#452a26",
      "--text-primary": "#eae2e1", "--text-secondary": "#a88580", "--accent-blue": "#bb4a3a", "--accent-green": "#3abb70",
      "--accent-purple": "#903abb", "--accent-gold": "#bb943a", "--accent-rose": "#bb3a4f", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-paprika", name: "Paprika", category: "standard", mode: "light", pairId: "paprika",
    preview: { bg: "#f9f6f6", fg: "#2a1f1d", accent: "#bb4a3a", secondary: "#f0ebea" },
    vars: {
      "--bg-primary": "#f9f6f6", "--bg-secondary": "#f0ebea", "--bg-tertiary": "#e4d9d8", "--bg-elevated": "#ffffff",
      "--text-primary": "#2a1f1d", "--text-secondary": "#765a56", "--accent-blue": "#bb4a3a", "--accent-green": "#3abb70",
      "--accent-purple": "#903abb", "--accent-gold": "#bb943a", "--accent-rose": "#bb3a4f", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Matte Rustic Pink ──
  {
    id: "dark-matte-rustic-pink", name: "Matte Rustic Pink", category: "standard", mode: "dark", pairId: "matte-rustic-pink",
    preview: { bg: "#16110e", fg: "#e9e5e2", accent: "#d1a489", secondary: "#231a15" },
    vars: {
      "--bg-primary": "#16110e", "--bg-secondary": "#231a15", "--bg-tertiary": "#33261f", "--bg-elevated": "#433229",
      "--text-primary": "#e9e5e2", "--text-secondary": "#a49083", "--accent-blue": "#d1a489", "--accent-green": "#89d1a7",
      "--accent-purple": "#b989d1", "--accent-gold": "#d1bc89", "--accent-rose": "#d18995", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-matte-rustic-pink", name: "Matte Rustic Pink", category: "standard", mode: "light", pairId: "matte-rustic-pink",
    preview: { bg: "#f9f7f6", fg: "#29221e", accent: "#7c4c30", secondary: "#f0ecea" },
    vars: {
      "--bg-primary": "#f9f7f6", "--bg-secondary": "#f0ecea", "--bg-tertiary": "#e3dcd9", "--bg-elevated": "#ffffff",
      "--text-primary": "#29221e", "--text-secondary": "#746258", "--accent-blue": "#7c4c30", "--accent-green": "#307c50",
      "--accent-purple": "#63307c", "--accent-gold": "#7c6530", "--accent-rose": "#7c303d", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Elderton ──
  {
    id: "dark-elderton", name: "Elderton", category: "standard", mode: "dark", pairId: "elderton",
    preview: { bg: "#131210", fg: "#e7e5e4", accent: "#a9734c", secondary: "#1e1c1a" },
    vars: {
      "--bg-primary": "#131210", "--bg-secondary": "#1e1c1a", "--bg-tertiary": "#2c2826", "--bg-elevated": "#3a3531",
      "--text-primary": "#e7e5e4", "--text-secondary": "#99938f", "--accent-blue": "#a9734c", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-elderton", name: "Elderton", category: "standard", mode: "light", pairId: "elderton",
    preview: { bg: "#f8f7f7", fg: "#252322", accent: "#a9734c", secondary: "#eeedec" },
    vars: {
      "--bg-primary": "#f8f7f7", "--bg-secondary": "#eeedec", "--bg-tertiary": "#dfdedc", "--bg-elevated": "#ffffff",
      "--text-primary": "#252322", "--text-secondary": "#6a6562", "--accent-blue": "#a9734c", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Viridis ──
  {
    id: "dark-viridis", name: "Viridis", category: "standard", mode: "dark", pairId: "viridis",
    preview: { bg: "#141310", fg: "#e7e7e4", accent: "#ccc88e", secondary: "#1f1f19" },
    vars: {
      "--bg-primary": "#141310", "--bg-secondary": "#1f1f19", "--bg-tertiary": "#2d2c25", "--bg-elevated": "#3b3a30",
      "--text-primary": "#e7e7e4", "--text-secondary": "#9b9a8d", "--accent-blue": "#ccc88e", "--accent-green": "#8ecca8",
      "--accent-purple": "#b88ecc", "--accent-gold": "#ccba8e", "--accent-rose": "#cc8e99", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-viridis", name: "Viridis", category: "standard", mode: "light", pairId: "viridis",
    preview: { bg: "#f8f8f7", fg: "#262621", accent: "#9a9445", secondary: "#eeeeec" },
    vars: {
      "--bg-primary": "#f8f8f7", "--bg-secondary": "#eeeeec", "--bg-tertiary": "#e0e0dc", "--bg-elevated": "#ffffff",
      "--text-primary": "#262621", "--text-secondary": "#6c6b60", "--accent-blue": "#9a9445", "--accent-green": "#459a69",
      "--accent-purple": "#7e459a", "--accent-gold": "#9a8145", "--accent-rose": "#9a4553", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Alizarin ──
  {
    id: "dark-alizarin", name: "Alizarin", category: "standard", mode: "dark", pairId: "alizarin",
    preview: { bg: "#141010", fg: "#e7e4e4", accent: "#a9554c", secondary: "#1f1919" },
    vars: {
      "--bg-primary": "#141010", "--bg-secondary": "#1f1919", "--bg-tertiary": "#2e2524", "--bg-elevated": "#3c312f",
      "--text-primary": "#e7e4e4", "--text-secondary": "#9c8e8c", "--accent-blue": "#a9554c", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-alizarin", name: "Alizarin", category: "standard", mode: "light", pairId: "alizarin",
    preview: { bg: "#f8f7f7", fg: "#262221", accent: "#a9554c", secondary: "#eeecec" },
    vars: {
      "--bg-primary": "#f8f7f7", "--bg-secondary": "#eeecec", "--bg-tertiary": "#e0dcdb", "--bg-elevated": "#ffffff",
      "--text-primary": "#262221", "--text-secondary": "#6c6160", "--accent-blue": "#a9554c", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Breathe ──
  {
    id: "dark-breathe", name: "Breathe", category: "standard", mode: "dark", pairId: "breathe",
    preview: { bg: "#111213", fg: "#e5e6e6", accent: "#579bb4", secondary: "#1a1d1e" },
    vars: {
      "--bg-primary": "#111213", "--bg-secondary": "#1a1d1e", "--bg-tertiary": "#262a2b", "--bg-elevated": "#323739",
      "--text-primary": "#e5e6e6", "--text-secondary": "#929596", "--accent-blue": "#579bb4", "--accent-green": "#57b47e",
      "--accent-purple": "#9557b4", "--accent-gold": "#b49857", "--accent-rose": "#b45767", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-breathe", name: "Breathe", category: "standard", mode: "light", pairId: "breathe",
    preview: { bg: "#f7f7f8", fg: "#232424", accent: "#4c90a9", secondary: "#ecedee" },
    vars: {
      "--bg-primary": "#f7f7f8", "--bg-secondary": "#ecedee", "--bg-tertiary": "#dddedf", "--bg-elevated": "#ffffff",
      "--text-primary": "#232424", "--text-secondary": "#646768", "--accent-blue": "#4c90a9", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Epoch ──
  {
    id: "dark-epoch", name: "Epoch", category: "standard", mode: "dark", pairId: "epoch",
    preview: { bg: "#141011", fg: "#e7e4e5", accent: "#a94c70", secondary: "#1f191b" },
    vars: {
      "--bg-primary": "#141011", "--bg-secondary": "#1f191b", "--bg-tertiary": "#2d2428", "--bg-elevated": "#3b3034",
      "--text-primary": "#e7e4e5", "--text-secondary": "#9b8d92", "--accent-blue": "#a94c70", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-epoch", name: "Epoch", category: "standard", mode: "light", pairId: "epoch",
    preview: { bg: "#f8f7f7", fg: "#262123", accent: "#853c58", secondary: "#eeeced" },
    vars: {
      "--bg-primary": "#f8f7f7", "--bg-secondary": "#eeeced", "--bg-tertiary": "#e0dcdd", "--bg-elevated": "#ffffff",
      "--text-primary": "#262123", "--text-secondary": "#6c6065", "--accent-blue": "#853c58", "--accent-green": "#3c855a",
      "--accent-purple": "#6d3c85", "--accent-gold": "#856f3c", "--accent-rose": "#853c48", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Adeline ──
  {
    id: "dark-adeline", name: "Adeline", category: "standard", mode: "dark", pairId: "adeline",
    preview: { bg: "#111312", fg: "#e5e6e6", accent: "#4ca97a", secondary: "#1a1e1c" },
    vars: {
      "--bg-primary": "#111312", "--bg-secondary": "#1a1e1c", "--bg-tertiary": "#262b29", "--bg-elevated": "#323936",
      "--text-primary": "#e5e6e6", "--text-secondary": "#909894", "--accent-blue": "#4ca97a", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-adeline", name: "Adeline", category: "standard", mode: "light", pairId: "adeline",
    preview: { bg: "#f7f8f7", fg: "#222524", accent: "#3f8c66", secondary: "#eceeed" },
    vars: {
      "--bg-primary": "#f7f8f7", "--bg-secondary": "#eceeed", "--bg-tertiary": "#dddfde", "--bg-elevated": "#ffffff",
      "--text-primary": "#222524", "--text-secondary": "#636966", "--accent-blue": "#3f8c66", "--accent-green": "#3f8c5f",
      "--accent-purple": "#723f8c", "--accent-gold": "#8c753f", "--accent-rose": "#8c3f4c", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Tiru ──
  {
    id: "dark-tiru", name: "Tiru", category: "standard", mode: "dark", pairId: "tiru",
    preview: { bg: "#0f1315", fg: "#e3e6e8", accent: "#4c8ba9", secondary: "#171e21" },
    vars: {
      "--bg-primary": "#0f1315", "--bg-secondary": "#171e21", "--bg-tertiary": "#222b30", "--bg-elevated": "#2c393f",
      "--text-primary": "#e3e6e8", "--text-secondary": "#8898a0", "--accent-blue": "#4c8ba9", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-tiru", name: "Tiru", category: "standard", mode: "light", pairId: "tiru",
    preview: { bg: "#f7f8f8", fg: "#202528", accent: "#437a95", secondary: "#ebeeef" },
    vars: {
      "--bg-primary": "#f7f8f8", "--bg-secondary": "#ebeeef", "--bg-tertiary": "#dadfe2", "--bg-elevated": "#ffffff",
      "--text-primary": "#202528", "--text-secondary": "#5c6970", "--accent-blue": "#437a95", "--accent-green": "#439565",
      "--accent-purple": "#7a4395", "--accent-gold": "#957d43", "--accent-rose": "#954351", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Penelope ──
  {
    id: "dark-penelope", name: "Penelope", category: "standard", mode: "dark", pairId: "penelope",
    preview: { bg: "#14100f", fg: "#e7e4e4", accent: "#cc988e", secondary: "#201918" },
    vars: {
      "--bg-primary": "#14100f", "--bg-secondary": "#201918", "--bg-tertiary": "#2f2523", "--bg-elevated": "#3d302e",
      "--text-primary": "#e7e4e4", "--text-secondary": "#9e8d8a", "--accent-blue": "#cc988e", "--accent-green": "#8ecca8",
      "--accent-purple": "#b88ecc", "--accent-gold": "#ccba8e", "--accent-rose": "#cc8e99", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-penelope", name: "Penelope", category: "standard", mode: "light", pairId: "penelope",
    preview: { bg: "#f8f7f7", fg: "#272120", accent: "#713c33", secondary: "#efecec" },
    vars: {
      "--bg-primary": "#f8f7f7", "--bg-secondary": "#efecec", "--bg-tertiary": "#e1dcdb", "--bg-elevated": "#ffffff",
      "--text-primary": "#272120", "--text-secondary": "#6e615e", "--accent-blue": "#713c33", "--accent-green": "#33714c",
      "--accent-purple": "#5c3371", "--accent-gold": "#715e33", "--accent-rose": "#71333d", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Raku ──
  {
    id: "dark-raku", name: "Raku", category: "standard", mode: "dark", pairId: "raku",
    preview: { bg: "#131110", fg: "#e7e5e4", accent: "#a95d4c", secondary: "#1e1b1a" },
    vars: {
      "--bg-primary": "#131110", "--bg-secondary": "#1e1b1a", "--bg-tertiary": "#2c2726", "--bg-elevated": "#3a3331",
      "--text-primary": "#e7e5e4", "--text-secondary": "#99918f", "--accent-blue": "#a95d4c", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-raku", name: "Raku", category: "standard", mode: "light", pairId: "raku",
    preview: { bg: "#f8f7f7", fg: "#252322", accent: "#965243", secondary: "#eeedec" },
    vars: {
      "--bg-primary": "#f8f7f7", "--bg-secondary": "#eeedec", "--bg-tertiary": "#dfdddc", "--bg-elevated": "#ffffff",
      "--text-primary": "#252322", "--text-secondary": "#6a6362", "--accent-blue": "#965243", "--accent-green": "#439666",
      "--accent-purple": "#7a4396", "--accent-gold": "#967d43", "--accent-rose": "#964351", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Thermal ──
  {
    id: "dark-thermal", name: "Thermal", category: "standard", mode: "dark", pairId: "thermal",
    preview: { bg: "#0f1215", fg: "#e3e6e8", accent: "#8eb0cc", secondary: "#171d21" },
    vars: {
      "--bg-primary": "#0f1215", "--bg-secondary": "#171d21", "--bg-tertiary": "#212a30", "--bg-elevated": "#2c3740",
      "--text-primary": "#e3e6e8", "--text-secondary": "#8795a1", "--accent-blue": "#8eb0cc", "--accent-green": "#8ecca8",
      "--accent-purple": "#b88ecc", "--accent-gold": "#ccba8e", "--accent-rose": "#cc8e99", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-thermal", name: "Thermal", category: "standard", mode: "light", pairId: "thermal",
    preview: { bg: "#f6f7f8", fg: "#1f2428", accent: "#335571", secondary: "#ebedef" },
    vars: {
      "--bg-primary": "#f6f7f8", "--bg-secondary": "#ebedef", "--bg-tertiary": "#dadee2", "--bg-elevated": "#ffffff",
      "--text-primary": "#1f2428", "--text-secondary": "#5c6770", "--accent-blue": "#335571", "--accent-green": "#33714c",
      "--accent-purple": "#5c3371", "--accent-gold": "#715e33", "--accent-rose": "#71333d", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Tiramisu ──
  {
    id: "dark-tiramisu", name: "Tiramisu", category: "standard", mode: "dark", pairId: "tiramisu",
    preview: { bg: "#131110", fg: "#e7e5e4", accent: "#ad694e", secondary: "#1e1b1a" },
    vars: {
      "--bg-primary": "#131110", "--bg-secondary": "#1e1b1a", "--bg-tertiary": "#2c2725", "--bg-elevated": "#3a3431",
      "--text-primary": "#e7e5e4", "--text-secondary": "#9a928e", "--accent-blue": "#ad694e", "--accent-green": "#4ead75",
      "--accent-purple": "#8d4ead", "--accent-gold": "#ad904e", "--accent-rose": "#ad4e5d", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-tiramisu", name: "Tiramisu", category: "standard", mode: "light", pairId: "tiramisu",
    preview: { bg: "#f8f7f7", fg: "#262322", accent: "#a9674c", secondary: "#eeedec" },
    vars: {
      "--bg-primary": "#f8f7f7", "--bg-secondary": "#eeedec", "--bg-tertiary": "#e0dddc", "--bg-elevated": "#ffffff",
      "--text-primary": "#262322", "--text-secondary": "#6b6461", "--accent-blue": "#a9674c", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Olive Branch ──
  {
    id: "dark-olive-branch", name: "Olive Branch", category: "standard", mode: "dark", pairId: "olive-branch",
    preview: { bg: "#121311", fg: "#e6e6e5", accent: "#99ae4e", secondary: "#1d1e1a" },
    vars: {
      "--bg-primary": "#121311", "--bg-secondary": "#1d1e1a", "--bg-tertiary": "#2a2b26", "--bg-elevated": "#373932",
      "--text-primary": "#e6e6e5", "--text-secondary": "#959692", "--accent-blue": "#99ae4e", "--accent-green": "#4eae76",
      "--accent-purple": "#8e4eae", "--accent-gold": "#ae914e", "--accent-rose": "#ae4e5e", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-olive-branch", name: "Olive Branch", category: "standard", mode: "light", pairId: "olive-branch",
    preview: { bg: "#f8f8f7", fg: "#242423", accent: "#95a94c", secondary: "#eeeeec" },
    vars: {
      "--bg-primary": "#f8f8f7", "--bg-secondary": "#eeeeec", "--bg-tertiary": "#dfdfdd", "--bg-elevated": "#ffffff",
      "--text-primary": "#242423", "--text-secondary": "#676864", "--accent-blue": "#95a94c", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Paper Clip ──
  {
    id: "dark-paper-clip", name: "Paper Clip", category: "standard", mode: "dark", pairId: "paper-clip",
    preview: { bg: "#131211", fg: "#e6e6e5", accent: "#ccb08e", secondary: "#1e1c1a" },
    vars: {
      "--bg-primary": "#131211", "--bg-secondary": "#1e1c1a", "--bg-tertiary": "#2b2926", "--bg-elevated": "#393632",
      "--text-primary": "#e6e6e5", "--text-secondary": "#989490", "--accent-blue": "#ccb08e", "--accent-green": "#8ecca8",
      "--accent-purple": "#b88ecc", "--accent-gold": "#ccba8e", "--accent-rose": "#cc8e99", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-paper-clip", name: "Paper Clip", category: "standard", mode: "light", pairId: "paper-clip",
    preview: { bg: "#f8f7f7", fg: "#252422", accent: "#87653d", secondary: "#eeedec" },
    vars: {
      "--bg-primary": "#f8f7f7", "--bg-secondary": "#eeedec", "--bg-tertiary": "#dfdedd", "--bg-elevated": "#ffffff",
      "--text-primary": "#252422", "--text-secondary": "#696663", "--accent-blue": "#87653d", "--accent-green": "#3d875c",
      "--accent-purple": "#6f3d87", "--accent-gold": "#87713d", "--accent-rose": "#873d49", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Salty Brine ──
  {
    id: "dark-salty-brine", name: "Salty Brine", category: "standard", mode: "dark", pairId: "salty-brine",
    preview: { bg: "#111312", fg: "#e5e6e5", accent: "#8ecca9", secondary: "#1a1e1c" },
    vars: {
      "--bg-primary": "#111312", "--bg-secondary": "#1a1e1c", "--bg-tertiary": "#262b28", "--bg-elevated": "#323935",
      "--text-primary": "#e5e6e5", "--text-secondary": "#909893", "--accent-blue": "#8ecca9", "--accent-green": "#8ecca8",
      "--accent-purple": "#b88ecc", "--accent-gold": "#ccba8e", "--accent-rose": "#cc8e99", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-salty-brine", name: "Salty Brine", category: "standard", mode: "light", pairId: "salty-brine",
    preview: { bg: "#f7f8f7", fg: "#222524", accent: "#3d885d", secondary: "#eceeed" },
    vars: {
      "--bg-primary": "#f7f8f7", "--bg-secondary": "#eceeed", "--bg-tertiary": "#dddfde", "--bg-elevated": "#ffffff",
      "--text-primary": "#222524", "--text-secondary": "#636966", "--accent-blue": "#3d885d", "--accent-green": "#3d885c",
      "--accent-purple": "#6f3d88", "--accent-gold": "#88723d", "--accent-rose": "#883d4a", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Byzantine Blue ──
  {
    id: "dark-byzantine-blue", name: "Byzantine Blue", category: "standard", mode: "dark", pairId: "byzantine-blue",
    preview: { bg: "#111113", fg: "#e5e5e6", accent: "#7c8bc4", secondary: "#1a1b1e" },
    vars: {
      "--bg-primary": "#111113", "--bg-secondary": "#1a1b1e", "--bg-tertiary": "#26272c", "--bg-elevated": "#323339",
      "--text-primary": "#e5e5e6", "--text-secondary": "#8f9199", "--accent-blue": "#7c8bc4", "--accent-green": "#7cc49a",
      "--accent-purple": "#ac7cc4", "--accent-gold": "#c4ae7c", "--accent-rose": "#c47c88", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-byzantine-blue", name: "Byzantine Blue", category: "standard", mode: "light", pairId: "byzantine-blue",
    preview: { bg: "#f7f7f8", fg: "#222325", accent: "#4c5fa9", secondary: "#ecedee" },
    vars: {
      "--bg-primary": "#f7f7f8", "--bg-secondary": "#ecedee", "--bg-tertiary": "#dcdddf", "--bg-elevated": "#ffffff",
      "--text-primary": "#222325", "--text-secondary": "#62646a", "--accent-blue": "#4c5fa9", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Cappuccino White ──
  {
    id: "dark-cappuccino-white", name: "Cappuccino White", category: "standard", mode: "dark", pairId: "cappuccino-white",
    preview: { bg: "#141310", fg: "#e7e6e4", accent: "#ccbc8e", secondary: "#201e18" },
    vars: {
      "--bg-primary": "#141310", "--bg-secondary": "#201e18", "--bg-tertiary": "#2e2b24", "--bg-elevated": "#3c392f",
      "--text-primary": "#e7e6e4", "--text-secondary": "#9d988b", "--accent-blue": "#ccbc8e", "--accent-green": "#8ecca8",
      "--accent-purple": "#b88ecc", "--accent-gold": "#ccba8e", "--accent-rose": "#cc8e99", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-cappuccino-white", name: "Cappuccino White", category: "standard", mode: "light", pairId: "cappuccino-white",
    preview: { bg: "#f8f8f7", fg: "#272521", accent: "#716033", secondary: "#efeeec" },
    vars: {
      "--bg-primary": "#f8f8f7", "--bg-secondary": "#efeeec", "--bg-tertiary": "#e1dfdb", "--bg-elevated": "#ffffff",
      "--text-primary": "#272521", "--text-secondary": "#6d695f", "--accent-blue": "#716033", "--accent-green": "#33714c",
      "--accent-purple": "#5c3371", "--accent-gold": "#715e33", "--accent-rose": "#71333d", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Deep Onyx ──
  {
    id: "dark-deep-onyx", name: "Deep Onyx", category: "standard", mode: "dark", pairId: "deep-onyx",
    preview: { bg: "#131112", fg: "#e6e5e6", accent: "#a94c7a", secondary: "#1e1a1c" },
    vars: {
      "--bg-primary": "#131112", "--bg-secondary": "#1e1a1c", "--bg-tertiary": "#2b2629", "--bg-elevated": "#393236",
      "--text-primary": "#e6e5e6", "--text-secondary": "#949394", "--accent-blue": "#a94c7a", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-deep-onyx", name: "Deep Onyx", category: "standard", mode: "light", pairId: "deep-onyx",
    preview: { bg: "#f8f7f7", fg: "#242424", accent: "#883d63", secondary: "#eeeced" },
    vars: {
      "--bg-primary": "#f8f7f7", "--bg-secondary": "#eeeced", "--bg-tertiary": "#dfddde", "--bg-elevated": "#ffffff",
      "--text-primary": "#242424", "--text-secondary": "#666666", "--accent-blue": "#883d63", "--accent-green": "#3d885c",
      "--accent-purple": "#6f3d88", "--accent-gold": "#88723d", "--accent-rose": "#883d4a", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Night Watch ──
  {
    id: "dark-night-watch", name: "Night Watch", category: "standard", mode: "dark", pairId: "night-watch",
    preview: { bg: "#111313", fg: "#e5e6e6", accent: "#4ca9a9", secondary: "#1a1e1e" },
    vars: {
      "--bg-primary": "#111313", "--bg-secondary": "#1a1e1e", "--bg-tertiary": "#262b2b", "--bg-elevated": "#323939",
      "--text-primary": "#e5e6e6", "--text-secondary": "#919797", "--accent-blue": "#4ca9a9", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-night-watch", name: "Night Watch", category: "standard", mode: "light", pairId: "night-watch",
    preview: { bg: "#f7f8f8", fg: "#232525", accent: "#419090", secondary: "#eceeee" },
    vars: {
      "--bg-primary": "#f7f8f8", "--bg-secondary": "#eceeee", "--bg-tertiary": "#dddfdf", "--bg-elevated": "#ffffff",
      "--text-primary": "#232525", "--text-secondary": "#646868", "--accent-blue": "#419090", "--accent-green": "#419062",
      "--accent-purple": "#764190", "--accent-gold": "#907841", "--accent-rose": "#90414e", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Vining Ivy ──
  {
    id: "dark-vining-ivy", name: "Vining Ivy", category: "standard", mode: "dark", pairId: "vining-ivy",
    preview: { bg: "#111313", fg: "#e5e6e6", accent: "#4c9fa9", secondary: "#1a1e1e" },
    vars: {
      "--bg-primary": "#111313", "--bg-secondary": "#1a1e1e", "--bg-tertiary": "#262b2c", "--bg-elevated": "#323939",
      "--text-primary": "#e5e6e6", "--text-secondary": "#8f9899", "--accent-blue": "#4c9fa9", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-vining-ivy", name: "Vining Ivy", category: "standard", mode: "light", pairId: "vining-ivy",
    preview: { bg: "#f7f8f8", fg: "#222525", accent: "#4c9fa9", secondary: "#eceeee" },
    vars: {
      "--bg-primary": "#f7f8f8", "--bg-secondary": "#eceeee", "--bg-tertiary": "#dcdfdf", "--bg-elevated": "#ffffff",
      "--text-primary": "#222525", "--text-secondary": "#62696a", "--accent-blue": "#4c9fa9", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Guacamole ──
  {
    id: "dark-guacamole", name: "Guacamole", category: "standard", mode: "dark", pairId: "guacamole",
    preview: { bg: "#131310", fg: "#e6e7e4", accent: "#a9bd6c", secondary: "#1d1e1a" },
    vars: {
      "--bg-primary": "#131310", "--bg-secondary": "#1d1e1a", "--bg-tertiary": "#2b2c25", "--bg-elevated": "#383a31",
      "--text-primary": "#e6e7e4", "--text-secondary": "#979a8e", "--accent-blue": "#a9bd6c", "--accent-green": "#6cbd8e",
      "--accent-purple": "#a26cbd", "--accent-gold": "#bda56c", "--accent-rose": "#bd6c7a", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-guacamole", name: "Guacamole", category: "standard", mode: "light", pairId: "guacamole",
    preview: { bg: "#f8f8f7", fg: "#252622", accent: "#92a94c", secondary: "#eeeeec" },
    vars: {
      "--bg-primary": "#f8f8f7", "--bg-secondary": "#eeeeec", "--bg-tertiary": "#dfe0dc", "--bg-elevated": "#ffffff",
      "--text-primary": "#252622", "--text-secondary": "#686b61", "--accent-blue": "#92a94c", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Aqua Fiesta ──
  {
    id: "dark-aqua-fiesta", name: "Aqua Fiesta", category: "standard", mode: "dark", pairId: "aqua-fiesta",
    preview: { bg: "#101414", fg: "#e4e7e7", accent: "#8ecccc", secondary: "#191f1f" },
    vars: {
      "--bg-primary": "#101414", "--bg-secondary": "#191f1f", "--bg-tertiary": "#242d2d", "--bg-elevated": "#303c3c",
      "--text-primary": "#e4e7e7", "--text-secondary": "#8c9c9c", "--accent-blue": "#8ecccc", "--accent-green": "#8ecca8",
      "--accent-purple": "#b88ecc", "--accent-gold": "#ccba8e", "--accent-rose": "#cc8e99", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-aqua-fiesta", name: "Aqua Fiesta", category: "standard", mode: "light", pairId: "aqua-fiesta",
    preview: { bg: "#f7f8f8", fg: "#212626", accent: "#439696", secondary: "#eceeee" },
    vars: {
      "--bg-primary": "#f7f8f8", "--bg-secondary": "#eceeee", "--bg-tertiary": "#dbe0e0", "--bg-elevated": "#ffffff",
      "--text-primary": "#212626", "--text-secondary": "#606c6c", "--accent-blue": "#439696", "--accent-green": "#439666",
      "--accent-purple": "#7a4396", "--accent-gold": "#967d43", "--accent-rose": "#964351", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Whirlwind ──
  {
    id: "dark-whirlwind", name: "Whirlwind", category: "standard", mode: "dark", pairId: "whirlwind",
    preview: { bg: "#111213", fg: "#e5e6e6", accent: "#8eb8cc", secondary: "#1a1d1e" },
    vars: {
      "--bg-primary": "#111213", "--bg-secondary": "#1a1d1e", "--bg-tertiary": "#262a2b", "--bg-elevated": "#323739",
      "--text-primary": "#e5e6e6", "--text-secondary": "#939495", "--accent-blue": "#8eb8cc", "--accent-green": "#8ecca8",
      "--accent-purple": "#b88ecc", "--accent-gold": "#ccba8e", "--accent-rose": "#cc8e99", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-whirlwind", name: "Whirlwind", category: "standard", mode: "light", pairId: "whirlwind",
    preview: { bg: "#f7f7f8", fg: "#232424", accent: "#39687f", secondary: "#ecedee" },
    vars: {
      "--bg-primary": "#f7f7f8", "--bg-secondary": "#ecedee", "--bg-tertiary": "#dddedf", "--bg-elevated": "#ffffff",
      "--text-primary": "#232424", "--text-secondary": "#656667", "--accent-blue": "#39687f", "--accent-green": "#397f56",
      "--accent-purple": "#68397f", "--accent-gold": "#7f6a39", "--accent-rose": "#7f3945", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Limitless ──
  {
    id: "dark-limitless", name: "Limitless", category: "standard", mode: "dark", pairId: "limitless",
    preview: { bg: "#17140d", fg: "#e9e7e2", accent: "#d8c083", secondary: "#242014" },
    vars: {
      "--bg-primary": "#17140d", "--bg-secondary": "#242014", "--bg-tertiary": "#342e1d", "--bg-elevated": "#453c26",
      "--text-primary": "#e9e7e2", "--text-secondary": "#a79c80", "--accent-blue": "#d8c083", "--accent-green": "#83d8a6",
      "--accent-purple": "#bb83d8", "--accent-gold": "#d8be83", "--accent-rose": "#d88391", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-limitless", name: "Limitless", category: "standard", mode: "light", pairId: "limitless",
    preview: { bg: "#f9f8f6", fg: "#2a271d", accent: "#7c6427", secondary: "#f0efea" },
    vars: {
      "--bg-primary": "#f9f8f6", "--bg-secondary": "#f0efea", "--bg-tertiary": "#e4e0d8", "--bg-elevated": "#ffffff",
      "--text-primary": "#2a271d", "--text-secondary": "#766d56", "--accent-blue": "#7c6427", "--accent-green": "#277c4b",
      "--accent-purple": "#60277c", "--accent-gold": "#7c6327", "--accent-rose": "#7c2735", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Purple Basil ──
  {
    id: "dark-purple-basil", name: "Purple Basil", category: "standard", mode: "dark", pairId: "purple-basil",
    preview: { bg: "#131112", fg: "#e6e5e6", accent: "#a94c80", secondary: "#1e1a1c" },
    vars: {
      "--bg-primary": "#131112", "--bg-secondary": "#1e1a1c", "--bg-tertiary": "#2b2629", "--bg-elevated": "#393236",
      "--text-primary": "#e6e5e6", "--text-secondary": "#989094", "--accent-blue": "#a94c80", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-purple-basil", name: "Purple Basil", category: "standard", mode: "light", pairId: "purple-basil",
    preview: { bg: "#f8f7f7", fg: "#252224", accent: "#974473", secondary: "#eeeced" },
    vars: {
      "--bg-primary": "#f8f7f7", "--bg-secondary": "#eeeced", "--bg-tertiary": "#dfddde", "--bg-elevated": "#ffffff",
      "--text-primary": "#252224", "--text-secondary": "#696366", "--accent-blue": "#974473", "--accent-green": "#449767",
      "--accent-purple": "#7b4497", "--accent-gold": "#977e44", "--accent-rose": "#974452", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Amethyst ── Cobalt/Aintree-style: a deep, saturated backdrop
  // (violet instead of Cobalt's navy or Aintree's racing green) with a
  // vivid, slightly mismatched jewel-tone accent set rather than one flat
  // hue throughout. `preview.accent` is #9d5fef (H≈266°, S≈82%) so it lands
  // in the "Purples & Violets" hue-family group, unlike the existing
  // "Purple Basil"/"Aintree" pairs whose signature accent hues land
  // elsewhere despite their names — see `hueFamily.ts`.
  {
    id: "dark-amethyst", name: "Amethyst", category: "standard", mode: "dark", pairId: "amethyst",
    preview: { bg: "#241b36", fg: "#ede6f7", accent: "#9d5fef", secondary: "#2f2444" },
    vars: {
      "--bg-primary": "#241b36", "--bg-secondary": "#2f2444", "--bg-tertiary": "#3a2d54", "--bg-elevated": "#453765",
      "--text-primary": "#ede6f7", "--text-secondary": "#a08cc2", "--accent-blue": "#7ecbff", "--accent-green": "#7cf2b0",
      "--accent-purple": "#9d5fef", "--accent-gold": "#ffd166", "--accent-rose": "#ff8fd1", "--border-color": "rgba(255, 255, 255, 0.08)",
      "--error-color": "#ff5c72",
    },
  },
  {
    id: "light-amethyst", name: "Amethyst", category: "standard", mode: "light", pairId: "amethyst",
    preview: { bg: "#f8f5fc", fg: "#241b36", accent: "#7a3fc7", secondary: "#ece3f5" },
    vars: {
      "--bg-primary": "#f8f5fc", "--bg-secondary": "#ece3f5", "--bg-tertiary": "#ddd0ec", "--bg-elevated": "#ffffff",
      "--text-primary": "#241b36", "--text-secondary": "#7a5c96", "--accent-blue": "#2f6fb0", "--accent-green": "#227a54",
      "--accent-purple": "#7a3fc7", "--accent-gold": "#b8860b", "--accent-rose": "#b23a72", "--border-color": "rgba(0, 0, 0, 0.07)",
      "--error-color": "#c23b56",
    },
  },
  // ── Pair: Garnet ── Same Cobalt/Aintree construction as Amethyst above —
  // deep saturated backdrop plus vivid jewel-tone accents — in a wine/
  // burgundy hue instead of violet. The hero color lives in `--accent-rose`
  // (a burgundy is a rose-red, not a purple) at H≈343°, which puts
  // `preview.accent` just short of this classifier's 350° "Reds & Crimsons"
  // cutoff — canonical burgundy (e.g. #800020) computes to H≈345° for the
  // same reason, so it lands in "Pinks, Roses & Magentas" alongside
  // "Purple Basil" rather than with the true reds. That's the honest
  // classification for this hue, not a bug to work around.
  {
    id: "dark-garnet", name: "Garnet", category: "standard", mode: "dark", pairId: "garnet",
    preview: { bg: "#2a0f16", fg: "#f3e3e7", accent: "#d63864", secondary: "#3a161f" },
    vars: {
      "--bg-primary": "#2a0f16", "--bg-secondary": "#3a161f", "--bg-tertiary": "#4a1f29", "--bg-elevated": "#5a2833",
      "--text-primary": "#f3e3e7", "--text-secondary": "#b98492", "--accent-blue": "#5ec8ff", "--accent-green": "#7fe0a0",
      "--accent-purple": "#c98bd1", "--accent-gold": "#f2c14e", "--accent-rose": "#d63864", "--border-color": "rgba(255, 255, 255, 0.08)",
      "--error-color": "#ff5a5a",
    },
  },
  {
    id: "light-garnet", name: "Garnet", category: "standard", mode: "light", pairId: "garnet",
    preview: { bg: "#faf3f5", fg: "#2a0f16", accent: "#a32c52", secondary: "#f1e2e6" },
    vars: {
      "--bg-primary": "#faf3f5", "--bg-secondary": "#f1e2e6", "--bg-tertiary": "#e3cdd3", "--bg-elevated": "#ffffff",
      "--text-primary": "#2a0f16", "--text-secondary": "#8a5a66", "--accent-blue": "#2f6fb0", "--accent-green": "#2f8b5c",
      "--accent-purple": "#8a4a94", "--accent-gold": "#b8860b", "--accent-rose": "#a32c52", "--border-color": "rgba(0, 0, 0, 0.07)",
      "--error-color": "#c23b3b",
    },
  },
  // ── Pair: Magenta ── Same Cobalt/Aintree construction as Amethyst and
  // Garnet above — deep saturated backdrop plus vivid jewel-tone accents —
  // pushed past Garnet's wine hue into true magenta/fuchsia. The hero color
  // lives in `--accent-rose` at H≈324°, which lands in "Pinks, Roses &
  // Magentas" alongside Garnet rather than "Purples & Violets" like
  // Amethyst — see `hueFamily.ts`.
  {
    id: "dark-magenta", name: "Magenta", category: "standard", mode: "dark", pairId: "magenta",
    preview: { bg: "#2b0f28", fg: "#f5e3f0", accent: "#e0399e", secondary: "#3a1638" },
    vars: {
      "--bg-primary": "#2b0f28", "--bg-secondary": "#3a1638", "--bg-tertiary": "#4a1f48", "--bg-elevated": "#5a2858",
      "--text-primary": "#f5e3f0", "--text-secondary": "#c084b8", "--accent-blue": "#6ec8ff", "--accent-green": "#7ce0b0",
      "--accent-purple": "#b15fef", "--accent-gold": "#ffd166", "--accent-rose": "#e0399e", "--border-color": "rgba(255, 255, 255, 0.08)",
      "--error-color": "#ff5c72",
    },
  },
  {
    id: "light-magenta", name: "Magenta", category: "standard", mode: "light", pairId: "magenta",
    preview: { bg: "#fdf3fa", fg: "#2b0f28", accent: "#a3237d", secondary: "#f5e3f0" },
    vars: {
      "--bg-primary": "#fdf3fa", "--bg-secondary": "#f5e3f0", "--bg-tertiary": "#ecd0e5", "--bg-elevated": "#ffffff",
      "--text-primary": "#2b0f28", "--text-secondary": "#96578c", "--accent-blue": "#2f6fb0", "--accent-green": "#227a54",
      "--accent-purple": "#8a3fc7", "--accent-gold": "#b8860b", "--accent-rose": "#a3237d", "--border-color": "rgba(0, 0, 0, 0.07)",
      "--error-color": "#c23b56",
    },
  },
  // ── Pair: Parchment ── Old parchment paper: warm sepia/tan backdrop with
  // charred, burnt-umber edges in the dark counterpart and a sealing-wax
  // terracotta hero in `--accent-rose`. Dark-mode `preview.accent` is
  // #d97a52 (H≈18°), landing in "Oranges, Rust & Browns" per `hueFamily.ts`
  // — the aged-paper hue this theme is named for.
  {
    id: "dark-parchment", name: "Parchment", category: "standard", mode: "dark", pairId: "parchment",
    preview: { bg: "#241b12", fg: "#ede0c4", accent: "#d97a52", secondary: "#33261a" },
    vars: {
      "--bg-primary": "#241b12", "--bg-secondary": "#33261a", "--bg-tertiary": "#443122", "--bg-elevated": "#54402c",
      "--text-primary": "#ede0c4", "--text-secondary": "#b99a6e", "--accent-blue": "#7ea3cf", "--accent-green": "#8fae5e",
      "--accent-purple": "#ab8cc4", "--accent-gold": "#d9b34d", "--accent-rose": "#d97a52", "--border-color": "rgba(255, 255, 255, 0.08)",
      "--error-color": "#ff6b4f",
    },
  },
  {
    id: "light-parchment", name: "Parchment", category: "standard", mode: "light", pairId: "parchment",
    preview: { bg: "#ede0c4", fg: "#3b2a18", accent: "#a8442e", secondary: "#e2cfa3" },
    vars: {
      "--bg-primary": "#ede0c4", "--bg-secondary": "#e2cfa3", "--bg-tertiary": "#d3ba82", "--bg-elevated": "#f6efdd",
      "--text-primary": "#3b2a18", "--text-secondary": "#7d5f3c", "--accent-blue": "#4a6fa0", "--accent-green": "#5c7a3c",
      "--accent-purple": "#7c5a8c", "--accent-gold": "#b8860b", "--accent-rose": "#a8442e", "--border-color": "rgba(59, 42, 24, 0.14)",
      "--error-color": "#a8271c",
    },
  },
  // ── Pair: Quietude ──
  {
    id: "dark-quietude", name: "Quietude", category: "standard", mode: "dark", pairId: "quietude",
    preview: { bg: "#111311", fg: "#e5e6e5", accent: "#8ecc9c", secondary: "#1a1e1b" },
    vars: {
      "--bg-primary": "#111311", "--bg-secondary": "#1a1e1b", "--bg-tertiary": "#262b27", "--bg-elevated": "#323934",
      "--text-primary": "#e5e6e5", "--text-secondary": "#929693", "--accent-blue": "#8ecc9c", "--accent-green": "#8ecca8",
      "--accent-purple": "#b88ecc", "--accent-gold": "#ccba8e", "--accent-rose": "#cc8e99", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-quietude", name: "Quietude", category: "standard", mode: "light", pairId: "quietude",
    preview: { bg: "#f7f8f7", fg: "#232423", accent: "#408e51", secondary: "#eceeed" },
    vars: {
      "--bg-primary": "#f7f8f7", "--bg-secondary": "#eceeed", "--bg-tertiary": "#dddfdd", "--bg-elevated": "#ffffff",
      "--text-primary": "#232423", "--text-secondary": "#646865", "--accent-blue": "#408e51", "--accent-green": "#408e61",
      "--accent-purple": "#74408e", "--accent-gold": "#8e7740", "--accent-rose": "#8e404d", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Persimmon ──
  {
    id: "dark-persimmon", name: "Persimmon", category: "standard", mode: "dark", pairId: "persimmon",
    preview: { bg: "#16110e", fg: "#e8e5e3", accent: "#c89d83", secondary: "#221b16" },
    vars: {
      "--bg-primary": "#16110e", "--bg-secondary": "#221b16", "--bg-tertiary": "#312720", "--bg-elevated": "#41332a",
      "--text-primary": "#e8e5e3", "--text-secondary": "#a29085", "--accent-blue": "#c89d83", "--accent-green": "#83c8a0",
      "--accent-purple": "#b183c8", "--accent-gold": "#c8b383", "--accent-rose": "#c8838e", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-persimmon", name: "Persimmon", category: "standard", mode: "light", pairId: "persimmon",
    preview: { bg: "#f8f7f6", fg: "#29231f", accent: "#a66d4a", secondary: "#f0edeb" },
    vars: {
      "--bg-primary": "#f8f7f6", "--bg-secondary": "#f0edeb", "--bg-tertiary": "#e2ddd9", "--bg-elevated": "#ffffff",
      "--text-primary": "#29231f", "--text-secondary": "#72635a", "--accent-blue": "#a66d4a", "--accent-green": "#4aa670",
      "--accent-purple": "#884aa6", "--accent-gold": "#a68b4a", "--accent-rose": "#a64a59", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Darkroom ──
  {
    id: "dark-darkroom", name: "Darkroom", category: "standard", mode: "dark", pairId: "darkroom",
    preview: { bg: "#131112", fg: "#e6e5e6", accent: "#a94c7a", secondary: "#1e1a1c" },
    vars: {
      "--bg-primary": "#131112", "--bg-secondary": "#1e1a1c", "--bg-tertiary": "#2b2629", "--bg-elevated": "#393236",
      "--text-primary": "#e6e5e6", "--text-secondary": "#959394", "--accent-blue": "#a94c7a", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-darkroom", name: "Darkroom", category: "standard", mode: "light", pairId: "darkroom",
    preview: { bg: "#f8f7f7", fg: "#242324", accent: "#833b5f", secondary: "#eeeced" },
    vars: {
      "--bg-primary": "#f8f7f7", "--bg-secondary": "#eeeced", "--bg-tertiary": "#dfddde", "--bg-elevated": "#ffffff",
      "--text-primary": "#242324", "--text-secondary": "#676566", "--accent-blue": "#833b5f", "--accent-green": "#3b8359",
      "--accent-purple": "#6b3b83", "--accent-gold": "#836d3b", "--accent-rose": "#833b47", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Aleutian ──
  {
    id: "dark-aleutian", name: "Aleutian", category: "standard", mode: "dark", pairId: "aleutian",
    preview: { bg: "#111213", fg: "#e4e5e7", accent: "#819ec7", secondary: "#1a1c1e" },
    vars: {
      "--bg-primary": "#111213", "--bg-secondary": "#1a1c1e", "--bg-tertiary": "#26282c", "--bg-elevated": "#323539",
      "--text-primary": "#e4e5e7", "--text-secondary": "#8f9399", "--accent-blue": "#819ec7", "--accent-green": "#81c79e",
      "--accent-purple": "#b081c7", "--accent-gold": "#c7b281", "--accent-rose": "#c7818d", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-aleutian", name: "Aleutian", category: "standard", mode: "light", pairId: "aleutian",
    preview: { bg: "#f7f7f8", fg: "#222325", accent: "#4b72a8", secondary: "#ecedee" },
    vars: {
      "--bg-primary": "#f7f7f8", "--bg-secondary": "#ecedee", "--bg-tertiary": "#dcdedf", "--bg-elevated": "#ffffff",
      "--text-primary": "#222325", "--text-secondary": "#62656a", "--accent-blue": "#4b72a8", "--accent-green": "#4ba872",
      "--accent-purple": "#894ba8", "--accent-gold": "#a88c4b", "--accent-rose": "#a84b5b", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Romance ──
  {
    id: "dark-romance", name: "Romance", category: "standard", mode: "dark", pairId: "romance",
    preview: { bg: "#15110f", fg: "#e8e5e3", accent: "#cca88e", secondary: "#201b18" },
    vars: {
      "--bg-primary": "#15110f", "--bg-secondary": "#201b18", "--bg-tertiary": "#2f2822", "--bg-elevated": "#3e342d",
      "--text-primary": "#e8e5e3", "--text-secondary": "#9e9289", "--accent-blue": "#cca88e", "--accent-green": "#8ecca8",
      "--accent-purple": "#b88ecc", "--accent-gold": "#ccba8e", "--accent-rose": "#cc8e99", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-romance", name: "Romance", category: "standard", mode: "light", pairId: "romance",
    preview: { bg: "#f8f7f7", fg: "#272320", accent: "#714d33", secondary: "#efedeb" },
    vars: {
      "--bg-primary": "#f8f7f7", "--bg-secondary": "#efedeb", "--bg-tertiary": "#e1dddb", "--bg-elevated": "#ffffff",
      "--text-primary": "#272320", "--text-secondary": "#6f655d", "--accent-blue": "#714d33", "--accent-green": "#33714d",
      "--accent-purple": "#5c3371", "--accent-gold": "#715f33", "--accent-rose": "#71333d", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Reflecting Pool ──
  {
    id: "dark-reflecting-pool", name: "Reflecting Pool", category: "standard", mode: "dark", pairId: "reflecting-pool",
    preview: { bg: "#101314", fg: "#e4e7e7", accent: "#7bbfc3", secondary: "#191e1f" },
    vars: {
      "--bg-primary": "#101314", "--bg-secondary": "#191e1f", "--bg-tertiary": "#252c2d", "--bg-elevated": "#303a3b",
      "--text-primary": "#e4e7e7", "--text-secondary": "#8d9a9b", "--accent-blue": "#7bbfc3", "--accent-green": "#7bc399",
      "--accent-purple": "#ab7bc3", "--accent-gold": "#c3ae7b", "--accent-rose": "#c37b87", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-reflecting-pool", name: "Reflecting Pool", category: "standard", mode: "light", pairId: "reflecting-pool",
    preview: { bg: "#f7f8f8", fg: "#212626", accent: "#4ca3a9", secondary: "#eceeee" },
    vars: {
      "--bg-primary": "#f7f8f8", "--bg-secondary": "#eceeee", "--bg-tertiary": "#dce0e0", "--bg-elevated": "#ffffff",
      "--text-primary": "#212626", "--text-secondary": "#616b6b", "--accent-blue": "#4ca3a9", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Honeycomb ──
  {
    id: "dark-honeycomb", name: "Honeycomb", category: "standard", mode: "dark", pairId: "honeycomb",
    preview: { bg: "#16120e", fg: "#e9e6e2", accent: "#c59c6a", secondary: "#231d15" },
    vars: {
      "--bg-primary": "#16120e", "--bg-secondary": "#231d15", "--bg-tertiary": "#332a1f", "--bg-elevated": "#423729",
      "--text-primary": "#e9e6e2", "--text-secondary": "#a49683", "--accent-blue": "#c59c6a", "--accent-green": "#6ac590",
      "--accent-purple": "#a76ac5", "--accent-gold": "#c5aa6a", "--accent-rose": "#c56a79", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-honeycomb", name: "Honeycomb", category: "standard", mode: "light", pairId: "honeycomb",
    preview: { bg: "#f9f7f6", fg: "#29241e", accent: "#b08045", secondary: "#f0edea" },
    vars: {
      "--bg-primary": "#f9f7f6", "--bg-secondary": "#f0edea", "--bg-tertiary": "#e3ded9", "--bg-elevated": "#ffffff",
      "--text-primary": "#29241e", "--text-secondary": "#736759", "--accent-blue": "#b08045", "--accent-green": "#45b071",
      "--accent-purple": "#8c45b0", "--accent-gold": "#b09045", "--accent-rose": "#b04557", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Blank Canvas ──
  {
    id: "dark-blank-canvas", name: "Blank Canvas", category: "standard", mode: "dark", pairId: "blank-canvas",
    preview: { bg: "#15140f", fg: "#e8e7e3", accent: "#ccc28e", secondary: "#201f18" },
    vars: {
      "--bg-primary": "#15140f", "--bg-secondary": "#201f18", "--bg-tertiary": "#2f2d22", "--bg-elevated": "#3e3b2d",
      "--text-primary": "#e8e7e3", "--text-secondary": "#9f9b89", "--accent-blue": "#ccc28e", "--accent-green": "#8ecca8",
      "--accent-purple": "#b88ecc", "--accent-gold": "#ccba8e", "--accent-rose": "#cc8e99", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-blank-canvas", name: "Blank Canvas", category: "standard", mode: "light", pairId: "blank-canvas",
    preview: { bg: "#f8f8f7", fg: "#272620", accent: "#716633", secondary: "#efeeeb" },
    vars: {
      "--bg-primary": "#f8f8f7", "--bg-secondary": "#efeeeb", "--bg-tertiary": "#e1e0db", "--bg-elevated": "#ffffff",
      "--text-primary": "#272620", "--text-secondary": "#6f6c5d", "--accent-blue": "#716633", "--accent-green": "#33714c",
      "--accent-purple": "#5c3371", "--accent-gold": "#715e33", "--accent-rose": "#71333d", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Blueprint ──
  {
    id: "dark-blueprint", name: "Blueprint", category: "standard", mode: "dark", pairId: "blueprint",
    preview: { bg: "#101214", fg: "#e4e6e7", accent: "#4c84a9", secondary: "#191d1f" },
    vars: {
      "--bg-primary": "#101214", "--bg-secondary": "#191d1f", "--bg-tertiary": "#252a2d", "--bg-elevated": "#30373b",
      "--text-primary": "#e4e6e7", "--text-secondary": "#8d959b", "--accent-blue": "#4c84a9", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-blueprint", name: "Blueprint", category: "standard", mode: "light", pairId: "blueprint",
    preview: { bg: "#f7f7f8", fg: "#212426", accent: "#4c84a9", secondary: "#ecedee" },
    vars: {
      "--bg-primary": "#f7f7f8", "--bg-secondary": "#ecedee", "--bg-tertiary": "#dcdee0", "--bg-elevated": "#ffffff",
      "--text-primary": "#212426", "--text-secondary": "#60676c", "--accent-blue": "#4c84a9", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: In the Moment ──
  {
    id: "dark-in-the-moment", name: "In the Moment", category: "standard", mode: "dark", pairId: "in-the-moment",
    preview: { bg: "#111313", fg: "#e5e6e6", accent: "#59aab4", secondary: "#1a1d1e" },
    vars: {
      "--bg-primary": "#111313", "--bg-secondary": "#1a1d1e", "--bg-tertiary": "#262b2b", "--bg-elevated": "#323839",
      "--text-primary": "#e5e6e6", "--text-secondary": "#919697", "--accent-blue": "#59aab4", "--accent-green": "#59b47f",
      "--accent-purple": "#9659b4", "--accent-gold": "#b49959", "--accent-rose": "#b45968", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-in-the-moment", name: "In the Moment", category: "standard", mode: "light", pairId: "in-the-moment",
    preview: { bg: "#f7f8f8", fg: "#232425", accent: "#4c9ea9", secondary: "#eceeee" },
    vars: {
      "--bg-primary": "#f7f8f8", "--bg-secondary": "#eceeee", "--bg-tertiary": "#dddfdf", "--bg-elevated": "#ffffff",
      "--text-primary": "#232425", "--text-secondary": "#646868", "--accent-blue": "#4c9ea9", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Poised Taupe ──
  {
    id: "dark-poised-taupe", name: "Poised Taupe", category: "standard", mode: "dark", pairId: "poised-taupe",
    preview: { bg: "#131211", fg: "#e6e5e5", accent: "#b17550", secondary: "#1e1c1a" },
    vars: {
      "--bg-primary": "#131211", "--bg-secondary": "#1e1c1a", "--bg-tertiary": "#2b2826", "--bg-elevated": "#393532",
      "--text-primary": "#e6e5e5", "--text-secondary": "#969392", "--accent-blue": "#b17550", "--accent-green": "#50b178",
      "--accent-purple": "#9150b1", "--accent-gold": "#b19450", "--accent-rose": "#b15060", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-poised-taupe", name: "Poised Taupe", category: "standard", mode: "light", pairId: "poised-taupe",
    preview: { bg: "#f8f7f7", fg: "#242423", accent: "#a9704c", secondary: "#eeedec" },
    vars: {
      "--bg-primary": "#f8f7f7", "--bg-secondary": "#eeedec", "--bg-tertiary": "#dfdedd", "--bg-elevated": "#ffffff",
      "--text-primary": "#242423", "--text-secondary": "#686664", "--accent-blue": "#a9704c", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Alabaster ──
  {
    id: "dark-alabaster", name: "Alabaster", category: "standard", mode: "dark", pairId: "alabaster",
    preview: { bg: "#141310", fg: "#e7e6e4", accent: "#ccbf8e", secondary: "#1f1e19" },
    vars: {
      "--bg-primary": "#141310", "--bg-secondary": "#1f1e19", "--bg-tertiary": "#2d2b25", "--bg-elevated": "#3b3930",
      "--text-primary": "#e7e6e4", "--text-secondary": "#9b988d", "--accent-blue": "#ccbf8e", "--accent-green": "#8ecca8",
      "--accent-purple": "#b88ecc", "--accent-gold": "#ccba8e", "--accent-rose": "#cc8e99", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-alabaster", name: "Alabaster", category: "standard", mode: "light", pairId: "alabaster",
    preview: { bg: "#f8f8f7", fg: "#262521", accent: "#716333", secondary: "#eeeeec" },
    vars: {
      "--bg-primary": "#f8f8f7", "--bg-secondary": "#eeeeec", "--bg-tertiary": "#e0dfdc", "--bg-elevated": "#ffffff",
      "--text-primary": "#262521", "--text-secondary": "#6c6960", "--accent-blue": "#716333", "--accent-green": "#33714c",
      "--accent-purple": "#5c3371", "--accent-gold": "#715e33", "--accent-rose": "#71333d", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Back to Nature ──
  {
    id: "dark-back-to-nature", name: "Back to Nature", category: "standard", mode: "dark", pairId: "back-to-nature",
    preview: { bg: "#141410", fg: "#e7e7e4", accent: "#ccc88e", secondary: "#1f1f19" },
    vars: {
      "--bg-primary": "#141410", "--bg-secondary": "#1f1f19", "--bg-tertiary": "#2d2d24", "--bg-elevated": "#3b3b30",
      "--text-primary": "#e7e7e4", "--text-secondary": "#9b9a8d", "--accent-blue": "#ccc88e", "--accent-green": "#8ecca8",
      "--accent-purple": "#b78ecc", "--accent-gold": "#ccb98e", "--accent-rose": "#cc8e98", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-back-to-nature", name: "Back to Nature", category: "standard", mode: "light", pairId: "back-to-nature",
    preview: { bg: "#f8f8f7", fg: "#262621", accent: "#9b9646", secondary: "#eeeeec" },
    vars: {
      "--bg-primary": "#f8f8f7", "--bg-secondary": "#eeeeec", "--bg-tertiary": "#e0e0dc", "--bg-elevated": "#ffffff",
      "--text-primary": "#262621", "--text-secondary": "#6c6b60", "--accent-blue": "#9b9646", "--accent-green": "#469b69",
      "--accent-purple": "#7f469b", "--accent-gold": "#9b8246", "--accent-rose": "#9b4654", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Canyon Dusk ──
  {
    id: "dark-canyon-dusk", name: "Canyon Dusk", category: "standard", mode: "dark", pairId: "canyon-dusk",
    preview: { bg: "#14120f", fg: "#e7e5e4", accent: "#c59d7d", secondary: "#201c18" },
    vars: {
      "--bg-primary": "#14120f", "--bg-secondary": "#201c18", "--bg-tertiary": "#2f2823", "--bg-elevated": "#3d352e",
      "--text-primary": "#e7e5e4", "--text-secondary": "#9d938a", "--accent-blue": "#c59d7d", "--accent-green": "#7dc59b",
      "--accent-purple": "#ad7dc5", "--accent-gold": "#c5af7d", "--accent-rose": "#c57d89", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-canyon-dusk", name: "Canyon Dusk", category: "standard", mode: "light", pairId: "canyon-dusk",
    preview: { bg: "#f8f7f7", fg: "#272321", accent: "#a9754c", secondary: "#efedec" },
    vars: {
      "--bg-primary": "#f8f7f7", "--bg-secondary": "#efedec", "--bg-tertiary": "#e1dddb", "--bg-elevated": "#ffffff",
      "--text-primary": "#272321", "--text-secondary": "#6e655e", "--accent-blue": "#a9754c", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Breezeway ──
  {
    id: "dark-breezeway", name: "Breezeway", category: "standard", mode: "dark", pairId: "breezeway",
    preview: { bg: "#111311", fg: "#e5e6e5", accent: "#8ecc8e", secondary: "#1a1e1a" },
    vars: {
      "--bg-primary": "#111311", "--bg-secondary": "#1a1e1a", "--bg-tertiary": "#262b26", "--bg-elevated": "#323932",
      "--text-primary": "#e5e6e5", "--text-secondary": "#929692", "--accent-blue": "#8ecc8e", "--accent-green": "#8ecca8",
      "--accent-purple": "#b88ecc", "--accent-gold": "#ccba8e", "--accent-rose": "#cc8e99", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-breezeway", name: "Breezeway", category: "standard", mode: "light", pairId: "breezeway",
    preview: { bg: "#f7f8f7", fg: "#232423", accent: "#387c38", secondary: "#eceeec" },
    vars: {
      "--bg-primary": "#f7f8f7", "--bg-secondary": "#eceeec", "--bg-tertiary": "#dddfdd", "--bg-elevated": "#ffffff",
      "--text-primary": "#232423", "--text-secondary": "#646864", "--accent-blue": "#387c38", "--accent-green": "#387c54",
      "--accent-purple": "#66387c", "--accent-gold": "#7c6838", "--accent-rose": "#7c3843", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Rumors ──
  {
    id: "dark-rumors", name: "Rumors", category: "standard", mode: "dark", pairId: "rumors",
    preview: { bg: "#141010", fg: "#e7e4e4", accent: "#a94c52", secondary: "#1f191a" },
    vars: {
      "--bg-primary": "#141010", "--bg-secondary": "#1f191a", "--bg-tertiary": "#2d2525", "--bg-elevated": "#3b3031",
      "--text-primary": "#e7e4e4", "--text-secondary": "#9b8d8e", "--accent-blue": "#a94c52", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-rumors", name: "Rumors", category: "standard", mode: "light", pairId: "rumors",
    preview: { bg: "#f8f7f7", fg: "#262122", accent: "#a44a4f", secondary: "#eeecec" },
    vars: {
      "--bg-primary": "#f8f7f7", "--bg-secondary": "#eeecec", "--bg-tertiary": "#e0dcdc", "--bg-elevated": "#ffffff",
      "--text-primary": "#262122", "--text-secondary": "#6c6061", "--accent-blue": "#a44a4f", "--accent-green": "#4aa470",
      "--accent-purple": "#864aa4", "--accent-gold": "#a4894a", "--accent-rose": "#a44a59", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Cracked Pepper ──
  {
    id: "dark-cracked-pepper", name: "Cracked Pepper", category: "standard", mode: "dark", pairId: "cracked-pepper",
    preview: { bg: "#131111", fg: "#e6e6e6", accent: "#a94c4c", secondary: "#1e1a1a" },
    vars: {
      "--bg-primary": "#131111", "--bg-secondary": "#1e1a1a", "--bg-tertiary": "#2b2626", "--bg-elevated": "#393232",
      "--text-primary": "#e6e6e6", "--text-secondary": "#949494", "--accent-blue": "#a94c4c", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-cracked-pepper", name: "Cracked Pepper", category: "standard", mode: "light", pairId: "cracked-pepper",
    preview: { bg: "#f8f7f7", fg: "#242424", accent: "#753434", secondary: "#eeecec" },
    vars: {
      "--bg-primary": "#f8f7f7", "--bg-secondary": "#eeecec", "--bg-tertiary": "#dfdddd", "--bg-elevated": "#ffffff",
      "--text-primary": "#242424", "--text-secondary": "#666666", "--accent-blue": "#753434", "--accent-green": "#34754f",
      "--accent-purple": "#5f3475", "--accent-gold": "#756134", "--accent-rose": "#75343f", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Cavern Clay ──
  {
    id: "dark-cavern-clay", name: "Cavern Clay", category: "standard", mode: "dark", pairId: "cavern-clay",
    preview: { bg: "#14110f", fg: "#e7e5e4", accent: "#ae694e", secondary: "#201a18" },
    vars: {
      "--bg-primary": "#14110f", "--bg-secondary": "#201a18", "--bg-tertiary": "#2e2623", "--bg-elevated": "#3d322e",
      "--text-primary": "#e7e5e4", "--text-secondary": "#9d908a", "--accent-blue": "#ae694e", "--accent-green": "#4eae76",
      "--accent-purple": "#8e4eae", "--accent-gold": "#ae914e", "--accent-rose": "#ae4e5e", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-cavern-clay", name: "Cavern Clay", category: "standard", mode: "light", pairId: "cavern-clay",
    preview: { bg: "#f8f7f7", fg: "#272221", accent: "#a9664c", secondary: "#efecec" },
    vars: {
      "--bg-primary": "#f8f7f7", "--bg-secondary": "#efecec", "--bg-tertiary": "#e1dddb", "--bg-elevated": "#ffffff",
      "--text-primary": "#272221", "--text-secondary": "#6e635e", "--accent-blue": "#a9664c", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Oceanside ──
  {
    id: "dark-oceanside", name: "Oceanside", category: "standard", mode: "dark", pairId: "oceanside",
    preview: { bg: "#0e1315", fg: "#e3e7e8", accent: "#4c8ca9", secondary: "#171e21" },
    vars: {
      "--bg-primary": "#0e1315", "--bg-secondary": "#171e21", "--bg-tertiary": "#212c31", "--bg-elevated": "#2b3940",
      "--text-primary": "#e3e7e8", "--text-secondary": "#8799a1", "--accent-blue": "#4c8ca9", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-oceanside", name: "Oceanside", category: "standard", mode: "light", pairId: "oceanside",
    preview: { bg: "#f6f8f8", fg: "#1f2528", accent: "#447d97", secondary: "#ebeeef" },
    vars: {
      "--bg-primary": "#f6f8f8", "--bg-secondary": "#ebeeef", "--bg-tertiary": "#dadfe2", "--bg-elevated": "#ffffff",
      "--text-primary": "#1f2528", "--text-secondary": "#5b6a71", "--accent-blue": "#447d97", "--accent-green": "#449766",
      "--accent-purple": "#7b4497", "--accent-gold": "#977e44", "--accent-rose": "#974451", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Naval ──
  {
    id: "dark-naval", name: "Naval", category: "standard", mode: "dark", pairId: "naval",
    preview: { bg: "#101114", fg: "#e4e5e7", accent: "#4c70a9", secondary: "#191b1f" },
    vars: {
      "--bg-primary": "#101114", "--bg-secondary": "#191b1f", "--bg-tertiary": "#25282d", "--bg-elevated": "#30343b",
      "--text-primary": "#e4e5e7", "--text-secondary": "#8d929b", "--accent-blue": "#4c70a9", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-naval", name: "Naval", category: "standard", mode: "light", pairId: "naval",
    preview: { bg: "#f7f7f8", fg: "#212326", accent: "#3a5682", secondary: "#ecedee" },
    vars: {
      "--bg-primary": "#f7f7f8", "--bg-secondary": "#ecedee", "--bg-tertiary": "#dcdde0", "--bg-elevated": "#ffffff",
      "--text-primary": "#212326", "--text-secondary": "#60656c", "--accent-blue": "#3a5682", "--accent-green": "#3a8258",
      "--accent-purple": "#6a3a82", "--accent-gold": "#826c3a", "--accent-rose": "#823a46", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Urbane Bronze ──
  {
    id: "dark-urbane-bronze", name: "Urbane Bronze", category: "standard", mode: "dark", pairId: "urbane-bronze",
    preview: { bg: "#131211", fg: "#e6e6e5", accent: "#a9814c", secondary: "#1e1c1a" },
    vars: {
      "--bg-primary": "#131211", "--bg-secondary": "#1e1c1a", "--bg-tertiary": "#2b2926", "--bg-elevated": "#393632",
      "--text-primary": "#e6e6e5", "--text-secondary": "#969492", "--accent-blue": "#a9814c", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-urbane-bronze", name: "Urbane Bronze", category: "standard", mode: "light", pairId: "urbane-bronze",
    preview: { bg: "#f8f7f7", fg: "#242423", accent: "#997545", secondary: "#eeedec" },
    vars: {
      "--bg-primary": "#f8f7f7", "--bg-secondary": "#eeedec", "--bg-tertiary": "#dfdedd", "--bg-elevated": "#ffffff",
      "--text-primary": "#242423", "--text-secondary": "#676665", "--accent-blue": "#997545", "--accent-green": "#459968",
      "--accent-purple": "#7d4599", "--accent-gold": "#998045", "--accent-rose": "#994553", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Evergreen Fog ──
  {
    id: "dark-evergreen-fog", name: "Evergreen Fog", category: "standard", mode: "dark", pairId: "evergreen-fog",
    preview: { bg: "#131311", fg: "#e6e6e5", accent: "#bbb468", secondary: "#1e1d1a" },
    vars: {
      "--bg-primary": "#131311", "--bg-secondary": "#1e1d1a", "--bg-tertiary": "#2b2b26", "--bg-elevated": "#393832",
      "--text-primary": "#e6e6e5", "--text-secondary": "#969592", "--accent-blue": "#bbb468", "--accent-green": "#68bb8b",
      "--accent-purple": "#9f68bb", "--accent-gold": "#bba268", "--accent-rose": "#bb6876", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-evergreen-fog", name: "Evergreen Fog", category: "standard", mode: "light", pairId: "evergreen-fog",
    preview: { bg: "#f8f8f7", fg: "#242423", accent: "#a9a04c", secondary: "#eeeeec" },
    vars: {
      "--bg-primary": "#f8f8f7", "--bg-secondary": "#eeeeec", "--bg-tertiary": "#dfdfdd", "--bg-elevated": "#ffffff",
      "--text-primary": "#242423", "--text-secondary": "#686764", "--accent-blue": "#a9a04c", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Redend Point ──
  {
    id: "dark-redend-point", name: "Redend Point", category: "standard", mode: "dark", pairId: "redend-point",
    preview: { bg: "#131210", fg: "#e7e5e4", accent: "#b98463", secondary: "#1e1c1a" },
    vars: {
      "--bg-primary": "#131210", "--bg-secondary": "#1e1c1a", "--bg-tertiary": "#2c2825", "--bg-elevated": "#3a3531",
      "--text-primary": "#e7e5e4", "--text-secondary": "#9a938e", "--accent-blue": "#b98463", "--accent-green": "#63b987",
      "--accent-purple": "#9c63b9", "--accent-gold": "#b99f63", "--accent-rose": "#b96371", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-redend-point", name: "Redend Point", category: "standard", mode: "light", pairId: "redend-point",
    preview: { bg: "#f8f7f7", fg: "#262322", accent: "#a96f4c", secondary: "#eeedec" },
    vars: {
      "--bg-primary": "#f8f7f7", "--bg-secondary": "#eeedec", "--bg-tertiary": "#e0dddc", "--bg-elevated": "#ffffff",
      "--text-primary": "#262322", "--text-secondary": "#6b6561", "--accent-blue": "#a96f4c", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Upward ──
  {
    id: "dark-upward", name: "Upward", category: "standard", mode: "dark", pairId: "upward",
    preview: { bg: "#111213", fg: "#e5e5e6", accent: "#8eabcc", secondary: "#1a1c1e" },
    vars: {
      "--bg-primary": "#111213", "--bg-secondary": "#1a1c1e", "--bg-tertiary": "#26292b", "--bg-elevated": "#323539",
      "--text-primary": "#e5e5e6", "--text-secondary": "#909498", "--accent-blue": "#8eabcc", "--accent-green": "#8ecca8",
      "--accent-purple": "#b88ecc", "--accent-gold": "#ccba8e", "--accent-rose": "#cc8e99", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-upward", name: "Upward", category: "standard", mode: "light", pairId: "upward",
    preview: { bg: "#f7f7f8", fg: "#222425", accent: "#355275", secondary: "#ecedee" },
    vars: {
      "--bg-primary": "#f7f7f8", "--bg-secondary": "#ecedee", "--bg-tertiary": "#dddedf", "--bg-elevated": "#ffffff",
      "--text-primary": "#222425", "--text-secondary": "#636669", "--accent-blue": "#355275", "--accent-green": "#357550",
      "--accent-purple": "#603575", "--accent-gold": "#756235", "--accent-rose": "#753540", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Grounded ──
  {
    id: "dark-grounded", name: "Grounded", category: "standard", mode: "dark", pairId: "grounded",
    preview: { bg: "#131210", fg: "#e7e5e4", accent: "#a9794c", secondary: "#1e1c1a" },
    vars: {
      "--bg-primary": "#131210", "--bg-secondary": "#1e1c1a", "--bg-tertiary": "#2c2925", "--bg-elevated": "#3a3531",
      "--text-primary": "#e7e5e4", "--text-secondary": "#9a948e", "--accent-blue": "#a9794c", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-grounded", name: "Grounded", category: "standard", mode: "light", pairId: "grounded",
    preview: { bg: "#f8f7f7", fg: "#262422", accent: "#a6764a", secondary: "#eeedec" },
    vars: {
      "--bg-primary": "#f8f7f7", "--bg-secondary": "#eeedec", "--bg-tertiary": "#e0dedc", "--bg-elevated": "#ffffff",
      "--text-primary": "#262422", "--text-secondary": "#6b6661", "--accent-blue": "#a6764a", "--accent-green": "#4aa670",
      "--accent-purple": "#874aa6", "--accent-gold": "#a68a4a", "--accent-rose": "#a64a5a", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Cloud Dancer ──
  {
    id: "dark-cloud-dancer", name: "Cloud Dancer", category: "standard", mode: "dark", pairId: "cloud-dancer",
    preview: { bg: "#131310", fg: "#e7e6e4", accent: "#ccc08e", secondary: "#1e1d1a" },
    vars: {
      "--bg-primary": "#131310", "--bg-secondary": "#1e1d1a", "--bg-tertiary": "#2c2b26", "--bg-elevated": "#3a3831",
      "--text-primary": "#e7e6e4", "--text-secondary": "#99978f", "--accent-blue": "#ccc08e", "--accent-green": "#8ecca8",
      "--accent-purple": "#b88ecc", "--accent-gold": "#ccba8e", "--accent-rose": "#cc8e99", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-cloud-dancer", name: "Cloud Dancer", category: "standard", mode: "light", pairId: "cloud-dancer",
    preview: { bg: "#f8f8f7", fg: "#252522", accent: "#716433", secondary: "#eeeeec" },
    vars: {
      "--bg-primary": "#f8f8f7", "--bg-secondary": "#eeeeec", "--bg-tertiary": "#e0dfdc", "--bg-elevated": "#ffffff",
      "--text-primary": "#252522", "--text-secondary": "#6a6962", "--accent-blue": "#716433", "--accent-green": "#33714c",
      "--accent-purple": "#5c3371", "--accent-gold": "#715e33", "--accent-rose": "#71333d", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Viva Magenta ──
  {
    id: "dark-viva-magenta", name: "Viva Magenta", category: "standard", mode: "dark", pairId: "viva-magenta",
    preview: { bg: "#160d0f", fg: "#e9e2e3", accent: "#b14453", secondary: "#231517" },
    vars: {
      "--bg-primary": "#160d0f", "--bg-secondary": "#231517", "--bg-tertiary": "#331f22", "--bg-elevated": "#43282c",
      "--text-primary": "#e9e2e3", "--text-secondary": "#a58388", "--accent-blue": "#b14453", "--accent-green": "#44b171",
      "--accent-purple": "#8d44b1", "--accent-gold": "#b19044", "--accent-rose": "#b14456", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-viva-magenta", name: "Viva Magenta", category: "standard", mode: "light", pairId: "viva-magenta",
    preview: { bg: "#f9f6f6", fg: "#291e20", accent: "#b14453", secondary: "#f0eaeb" },
    vars: {
      "--bg-primary": "#f9f6f6", "--bg-secondary": "#f0eaeb", "--bg-tertiary": "#e3d9da", "--bg-elevated": "#ffffff",
      "--text-primary": "#291e20", "--text-secondary": "#74585c", "--accent-blue": "#b14453", "--accent-green": "#44b171",
      "--accent-purple": "#8d44b1", "--accent-gold": "#b19044", "--accent-rose": "#b14456", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Very Peri ──
  {
    id: "dark-very-peri", name: "Very Peri", category: "standard", mode: "dark", pairId: "very-peri",
    preview: { bg: "#0f1014", fg: "#e4e4e7", accent: "#555ab3", secondary: "#181920" },
    vars: {
      "--bg-primary": "#0f1014", "--bg-secondary": "#181920", "--bg-tertiary": "#23242e", "--bg-elevated": "#2e2f3d",
      "--text-primary": "#e4e4e7", "--text-secondary": "#8b8c9d", "--accent-blue": "#555ab3", "--accent-green": "#55b37c",
      "--accent-purple": "#9455b3", "--accent-gold": "#b39755", "--accent-rose": "#b35565", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-very-peri", name: "Very Peri", category: "standard", mode: "light", pairId: "very-peri",
    preview: { bg: "#f7f7f8", fg: "#212127", accent: "#4c51a9", secondary: "#ececef" },
    vars: {
      "--bg-primary": "#f7f7f8", "--bg-secondary": "#ececef", "--bg-tertiary": "#dbdbe1", "--bg-elevated": "#ffffff",
      "--text-primary": "#212127", "--text-secondary": "#5f5f6d", "--accent-blue": "#4c51a9", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Beacon ──
  {
    id: "dark-beacon", name: "Beacon", category: "standard", mode: "dark", pairId: "beacon",
    preview: { bg: "#17160c", fg: "#eae9e1", accent: "#eedc6c", secondary: "#242214" },
    vars: {
      "--bg-primary": "#17160c", "--bg-secondary": "#242214", "--bg-tertiary": "#35321d", "--bg-elevated": "#464125",
      "--text-primary": "#eae9e1", "--text-secondary": "#afa779", "--accent-blue": "#eedc6c", "--accent-green": "#6ceea3",
      "--accent-purple": "#c36cee", "--accent-gold": "#eec76c", "--accent-rose": "#ee6c82", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-beacon", name: "Beacon", category: "standard", mode: "light", pairId: "beacon",
    preview: { bg: "#f9f8f6", fg: "#2c291c", accent: "#bca415", secondary: "#f0efea" },
    vars: {
      "--bg-primary": "#f9f8f6", "--bg-secondary": "#f0efea", "--bg-tertiary": "#e4e2d8", "--bg-elevated": "#ffffff",
      "--text-primary": "#2c291c", "--text-secondary": "#7a7552", "--accent-blue": "#bca415", "--accent-green": "#15bc5b",
      "--accent-purple": "#8415bc", "--accent-gold": "#bc8a15", "--accent-rose": "#bc1531", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Classic Blue ──
  {
    id: "dark-classic-blue", name: "Classic Blue", category: "standard", mode: "dark", pairId: "classic-blue",
    preview: { bg: "#0d1116", fg: "#e2e5e9", accent: "#4271b3", secondary: "#151b23" },
    vars: {
      "--bg-primary": "#0d1116", "--bg-secondary": "#151b23", "--bg-tertiary": "#1e2733", "--bg-elevated": "#283343",
      "--text-primary": "#e2e5e9", "--text-secondary": "#8391a5", "--accent-blue": "#4271b3", "--accent-green": "#42b371",
      "--accent-purple": "#8d42b3", "--accent-gold": "#b39142", "--accent-rose": "#b34255", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-classic-blue", name: "Classic Blue", category: "standard", mode: "light", pairId: "classic-blue",
    preview: { bg: "#f6f7f9", fg: "#1e2329", accent: "#3d69a7", secondary: "#eaedf0" },
    vars: {
      "--bg-primary": "#f6f7f9", "--bg-secondary": "#eaedf0", "--bg-tertiary": "#d9dde3", "--bg-elevated": "#ffffff",
      "--text-primary": "#1e2329", "--text-secondary": "#586474", "--accent-blue": "#3d69a7", "--accent-green": "#3da769",
      "--accent-purple": "#843da7", "--accent-gold": "#a7873d", "--accent-rose": "#a73d4f", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Quartz ──
  {
    id: "dark-quartz", name: "Quartz", category: "standard", mode: "dark", pairId: "quartz",
    preview: { bg: "#160f0e", fg: "#e8e3e3", accent: "#cd928e", secondary: "#221716" },
    vars: {
      "--bg-primary": "#160f0e", "--bg-secondary": "#221716", "--bg-tertiary": "#312120", "--bg-elevated": "#412c2a",
      "--text-primary": "#e8e3e3", "--text-secondary": "#a28785", "--accent-blue": "#cd928e", "--accent-green": "#8ecda8",
      "--accent-purple": "#b88ecd", "--accent-gold": "#cdba8e", "--accent-rose": "#cd8e98", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-quartz", name: "Quartz", category: "standard", mode: "light", pairId: "quartz",
    preview: { bg: "#f8f6f6", fg: "#29201f", accent: "#713632", secondary: "#f0ebeb" },
    vars: {
      "--bg-primary": "#f8f6f6", "--bg-secondary": "#f0ebeb", "--bg-tertiary": "#e2dad9", "--bg-elevated": "#ffffff",
      "--text-primary": "#29201f", "--text-secondary": "#725c5a", "--accent-blue": "#713632", "--accent-green": "#32714c",
      "--accent-purple": "#5c3271", "--accent-gold": "#715e32", "--accent-rose": "#71323d", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Greenery ──
  {
    id: "dark-greenery", name: "Greenery", category: "standard", mode: "dark", pairId: "greenery",
    preview: { bg: "#13150f", fg: "#e6e8e3", accent: "#9ab65c", secondary: "#1e2117" },
    vars: {
      "--bg-primary": "#13150f", "--bg-secondary": "#1e2117", "--bg-tertiary": "#2b3022", "--bg-elevated": "#393f2c",
      "--text-primary": "#e6e8e3", "--text-secondary": "#98a088", "--accent-blue": "#9ab65c", "--accent-green": "#5cb682",
      "--accent-purple": "#985cb6", "--accent-gold": "#b69b5c", "--accent-rose": "#b65c6b", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-greenery", name: "Greenery", category: "standard", mode: "light", pairId: "greenery",
    preview: { bg: "#f8f8f7", fg: "#252820", accent: "#8ca94c", secondary: "#eeefeb" },
    vars: {
      "--bg-primary": "#f8f8f7", "--bg-secondary": "#eeefeb", "--bg-tertiary": "#dfe1da", "--bg-elevated": "#ffffff",
      "--text-primary": "#252820", "--text-secondary": "#6a705c", "--accent-blue": "#8ca94c", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Ultra Violet ──
  {
    id: "dark-ultra-violet", name: "Ultra Violet", category: "standard", mode: "dark", pairId: "ultra-violet",
    preview: { bg: "#100f14", fg: "#e4e4e7", accent: "#604ca9", secondary: "#1a1820" },
    vars: {
      "--bg-primary": "#100f14", "--bg-secondary": "#1a1820", "--bg-tertiary": "#26232e", "--bg-elevated": "#312e3d",
      "--text-primary": "#e4e4e7", "--text-secondary": "#8f8b9d", "--accent-blue": "#604ca9", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-ultra-violet", name: "Ultra Violet", category: "standard", mode: "light", pairId: "ultra-violet",
    preview: { bg: "#f7f7f8", fg: "#222127", accent: "#604ca9", secondary: "#ececef" },
    vars: {
      "--bg-primary": "#f7f7f8", "--bg-secondary": "#ececef", "--bg-tertiary": "#dcdbe1", "--bg-elevated": "#ffffff",
      "--text-primary": "#222127", "--text-secondary": "#625e6e", "--accent-blue": "#604ca9", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Living Coral ──
  {
    id: "dark-living-coral", name: "Living Coral", category: "standard", mode: "dark", pairId: "living-coral",
    preview: { bg: "#170e0c", fg: "#eae2e1", accent: "#e07f6d", secondary: "#241614" },
    vars: {
      "--bg-primary": "#170e0c", "--bg-secondary": "#241614", "--bg-tertiary": "#35201d", "--bg-elevated": "#462b25",
      "--text-primary": "#eae2e1", "--text-secondary": "#ac837c", "--accent-blue": "#e07f6d", "--accent-green": "#6de09d",
      "--accent-purple": "#ba6de0", "--accent-gold": "#e0be6d", "--accent-rose": "#e06d80", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-living-coral", name: "Living Coral", category: "standard", mode: "light", pairId: "living-coral",
    preview: { bg: "#f9f6f6", fg: "#2c1e1c", accent: "#c4422a", secondary: "#f0ebea" },
    vars: {
      "--bg-primary": "#f9f6f6", "--bg-secondary": "#f0ebea", "--bg-tertiary": "#e4dad8", "--bg-elevated": "#ffffff",
      "--text-primary": "#2c1e1c", "--text-secondary": "#7a5852", "--accent-blue": "#c4422a", "--accent-green": "#2ac46a",
      "--accent-purple": "#912ac4", "--accent-gold": "#c4962a", "--accent-rose": "#c42a44", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Cinnamon Slate ──
  {
    id: "dark-cinnamon-slate", name: "Cinnamon Slate", category: "standard", mode: "dark", pairId: "cinnamon-slate",
    preview: { bg: "#131111", fg: "#e6e5e5", accent: "#ab594d", secondary: "#1e1b1a" },
    vars: {
      "--bg-primary": "#131111", "--bg-secondary": "#1e1b1a", "--bg-tertiary": "#2b2726", "--bg-elevated": "#393332",
      "--text-primary": "#e6e5e5", "--text-secondary": "#969291", "--accent-blue": "#ab594d", "--accent-green": "#4dab74",
      "--accent-purple": "#8c4dab", "--accent-gold": "#ab8f4d", "--accent-rose": "#ab4d5d", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-cinnamon-slate", name: "Cinnamon Slate", category: "standard", mode: "light", pairId: "cinnamon-slate",
    preview: { bg: "#f8f7f7", fg: "#252323", accent: "#a9584c", secondary: "#eeedec" },
    vars: {
      "--bg-primary": "#f8f7f7", "--bg-secondary": "#eeedec", "--bg-tertiary": "#dfdddd", "--bg-elevated": "#ffffff",
      "--text-primary": "#252323", "--text-secondary": "#686564", "--accent-blue": "#a9584c", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Blue Nova ──
  {
    id: "dark-blue-nova", name: "Blue Nova", category: "standard", mode: "dark", pairId: "blue-nova",
    preview: { bg: "#101114", fg: "#e4e5e7", accent: "#4c66a9", secondary: "#191b1f" },
    vars: {
      "--bg-primary": "#101114", "--bg-secondary": "#191b1f", "--bg-tertiary": "#25272d", "--bg-elevated": "#30333b",
      "--text-primary": "#e4e5e7", "--text-secondary": "#8d919b", "--accent-blue": "#4c66a9", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-blue-nova", name: "Blue Nova", category: "standard", mode: "light", pairId: "blue-nova",
    preview: { bg: "#f7f7f8", fg: "#212326", accent: "#4c66a9", secondary: "#ecedee" },
    vars: {
      "--bg-primary": "#f7f7f8", "--bg-secondary": "#ecedee", "--bg-tertiary": "#dcdde0", "--bg-elevated": "#ffffff",
      "--text-primary": "#212326", "--text-secondary": "#60646c", "--accent-blue": "#4c66a9", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Raspberry Blush ──
  {
    id: "dark-raspberry-blush", name: "Raspberry Blush", category: "standard", mode: "dark", pairId: "raspberry-blush",
    preview: { bg: "#160f0e", fg: "#e9e3e2", accent: "#bd6c61", secondary: "#221716" },
    vars: {
      "--bg-primary": "#160f0e", "--bg-secondary": "#221716", "--bg-tertiary": "#322220", "--bg-elevated": "#422c29",
      "--text-primary": "#e9e3e2", "--text-secondary": "#a38885", "--accent-blue": "#bd6c61", "--accent-green": "#61bd87",
      "--accent-purple": "#9e61bd", "--accent-gold": "#bda161", "--accent-rose": "#bd6170", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-raspberry-blush", name: "Raspberry Blush", category: "standard", mode: "light", pairId: "raspberry-blush",
    preview: { bg: "#f8f7f6", fg: "#29201f", accent: "#ad5448", secondary: "#f0ebeb" },
    vars: {
      "--bg-primary": "#f8f7f6", "--bg-secondary": "#f0ebeb", "--bg-tertiary": "#e3dad9", "--bg-elevated": "#ffffff",
      "--text-primary": "#29201f", "--text-secondary": "#735c59", "--accent-blue": "#ad5448", "--accent-green": "#48ad72",
      "--accent-purple": "#8b48ad", "--accent-gold": "#ad8f48", "--accent-rose": "#ad4859", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: October Mist ──
  {
    id: "dark-october-mist", name: "October Mist", category: "standard", mode: "dark", pairId: "october-mist",
    preview: { bg: "#131311", fg: "#e6e6e5", accent: "#c9cc8e", secondary: "#1e1e1a" },
    vars: {
      "--bg-primary": "#131311", "--bg-secondary": "#1e1e1a", "--bg-tertiary": "#2b2b26", "--bg-elevated": "#383932",
      "--text-primary": "#e6e6e5", "--text-secondary": "#979890", "--accent-blue": "#c9cc8e", "--accent-green": "#8ecca8",
      "--accent-purple": "#b88ecc", "--accent-gold": "#ccba8e", "--accent-rose": "#cc8e99", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-october-mist", name: "October Mist", category: "standard", mode: "light", pairId: "october-mist",
    preview: { bg: "#f8f8f7", fg: "#252522", accent: "#929744", secondary: "#eeeeec" },
    vars: {
      "--bg-primary": "#f8f8f7", "--bg-secondary": "#eeeeec", "--bg-tertiary": "#dfdfdd", "--bg-elevated": "#ffffff",
      "--text-primary": "#252522", "--text-secondary": "#696963", "--accent-blue": "#929744", "--accent-green": "#449767",
      "--accent-purple": "#7b4497", "--accent-gold": "#977e44", "--accent-rose": "#974452", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Aegean Teal ──
  {
    id: "dark-aegean-teal", name: "Aegean Teal", category: "standard", mode: "dark", pairId: "aegean-teal",
    preview: { bg: "#111313", fg: "#e5e6e6", accent: "#52b1b2", secondary: "#1a1e1e" },
    vars: {
      "--bg-primary": "#111313", "--bg-secondary": "#1a1e1e", "--bg-tertiary": "#262b2b", "--bg-elevated": "#323939",
      "--text-primary": "#e5e6e6", "--text-secondary": "#929696", "--accent-blue": "#52b1b2", "--accent-green": "#52b27a",
      "--accent-purple": "#9252b2", "--accent-gold": "#b29552", "--accent-rose": "#b25262", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-aegean-teal", name: "Aegean Teal", category: "standard", mode: "light", pairId: "aegean-teal",
    preview: { bg: "#f7f8f8", fg: "#232424", accent: "#4ca9a9", secondary: "#eceeee" },
    vars: {
      "--bg-primary": "#f7f8f8", "--bg-secondary": "#eceeee", "--bg-tertiary": "#dddfdf", "--bg-elevated": "#ffffff",
      "--text-primary": "#232424", "--text-secondary": "#646868", "--accent-blue": "#4ca9a9", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: First Light ──
  {
    id: "dark-first-light", name: "First Light", category: "standard", mode: "dark", pairId: "first-light",
    preview: { bg: "#141110", fg: "#e7e5e4", accent: "#cc9c8e", secondary: "#1f1b19" },
    vars: {
      "--bg-primary": "#141110", "--bg-secondary": "#1f1b19", "--bg-tertiary": "#2d2725", "--bg-elevated": "#3b3330",
      "--text-primary": "#e7e5e4", "--text-secondary": "#9b908d", "--accent-blue": "#cc9c8e", "--accent-green": "#8ecca8",
      "--accent-purple": "#b88ecc", "--accent-gold": "#ccba8e", "--accent-rose": "#cc8e99", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-first-light", name: "First Light", category: "standard", mode: "light", pairId: "first-light",
    preview: { bg: "#f8f7f7", fg: "#262221", accent: "#714033", secondary: "#eeedec" },
    vars: {
      "--bg-primary": "#f8f7f7", "--bg-secondary": "#eeedec", "--bg-tertiary": "#e0dddc", "--bg-elevated": "#ffffff",
      "--text-primary": "#262221", "--text-secondary": "#6b6361", "--accent-blue": "#714033", "--accent-green": "#33714c",
      "--accent-purple": "#5c3371", "--accent-gold": "#715e33", "--accent-rose": "#71333d", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Metropolitan ──
  {
    id: "dark-metropolitan", name: "Metropolitan", category: "standard", mode: "dark", pairId: "metropolitan",
    preview: { bg: "#121311", fg: "#e6e6e5", accent: "#adcc8e", secondary: "#1c1e1a" },
    vars: {
      "--bg-primary": "#121311", "--bg-secondary": "#1c1e1a", "--bg-tertiary": "#292b26", "--bg-elevated": "#363932",
      "--text-primary": "#e6e6e5", "--text-secondary": "#949692", "--accent-blue": "#adcc8e", "--accent-green": "#8ecca8",
      "--accent-purple": "#b88ecc", "--accent-gold": "#ccba8e", "--accent-rose": "#cc8e99", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-metropolitan", name: "Metropolitan", category: "standard", mode: "light", pairId: "metropolitan",
    preview: { bg: "#f7f8f7", fg: "#242423", accent: "#64893e", secondary: "#edeeec" },
    vars: {
      "--bg-primary": "#f7f8f7", "--bg-secondary": "#edeeec", "--bg-tertiary": "#dedfdd", "--bg-elevated": "#ffffff",
      "--text-primary": "#242423", "--text-secondary": "#666765", "--accent-blue": "#64893e", "--accent-green": "#3e895d",
      "--accent-purple": "#703e89", "--accent-gold": "#89733e", "--accent-rose": "#893e4a", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Caliente ──
  {
    id: "dark-caliente", name: "Caliente", category: "standard", mode: "dark", pairId: "caliente",
    preview: { bg: "#160e0e", fg: "#e9e3e2", accent: "#ae4d47", secondary: "#231616" },
    vars: {
      "--bg-primary": "#160e0e", "--bg-secondary": "#231616", "--bg-tertiary": "#32201f", "--bg-elevated": "#422b29",
      "--text-primary": "#e9e3e2", "--text-secondary": "#a48684", "--accent-blue": "#ae4d47", "--accent-green": "#47ae72",
      "--accent-purple": "#8c47ae", "--accent-gold": "#ae8f47", "--accent-rose": "#ae4758", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-caliente", name: "Caliente", category: "standard", mode: "light", pairId: "caliente",
    preview: { bg: "#f8f6f6", fg: "#291f1e", accent: "#a24742", secondary: "#f0ebeb" },
    vars: {
      "--bg-primary": "#f8f6f6", "--bg-secondary": "#f0ebeb", "--bg-tertiary": "#e3dad9", "--bg-elevated": "#ffffff",
      "--text-primary": "#291f1e", "--text-secondary": "#735b59", "--accent-blue": "#a24742", "--accent-green": "#42a26a",
      "--accent-purple": "#8242a2", "--accent-gold": "#a28542", "--accent-rose": "#a24252", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Shadow ──
  {
    id: "dark-shadow", name: "Shadow", category: "standard", mode: "dark", pairId: "shadow",
    preview: { bg: "#121113", fg: "#e6e5e6", accent: "#7a4ca9", secondary: "#1c1a1e" },
    vars: {
      "--bg-primary": "#121113", "--bg-secondary": "#1c1a1e", "--bg-tertiary": "#29262b", "--bg-elevated": "#363239",
      "--text-primary": "#e6e5e6", "--text-secondary": "#949296", "--accent-blue": "#7a4ca9", "--accent-green": "#4ca973",
      "--accent-purple": "#8a4ca9", "--accent-gold": "#a98d4c", "--accent-rose": "#a94c5b", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-shadow", name: "Shadow", category: "standard", mode: "light", pairId: "shadow",
    preview: { bg: "#f7f7f8", fg: "#242324", accent: "#6f4599", secondary: "#edecee" },
    vars: {
      "--bg-primary": "#f7f7f8", "--bg-secondary": "#edecee", "--bg-tertiary": "#dedddf", "--bg-elevated": "#ffffff",
      "--text-primary": "#242324", "--text-secondary": "#666468", "--accent-blue": "#6f4599", "--accent-green": "#459968",
      "--accent-purple": "#7d4599", "--accent-gold": "#997f45", "--accent-rose": "#994553", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Simply White ──
  {
    id: "dark-simply-white", name: "Simply White", category: "standard", mode: "dark", pairId: "simply-white",
    preview: { bg: "#15140f", fg: "#e8e7e3", accent: "#ccc58e", secondary: "#212017" },
    vars: {
      "--bg-primary": "#15140f", "--bg-secondary": "#212017", "--bg-tertiary": "#302e21", "--bg-elevated": "#3f3d2c",
      "--text-primary": "#e8e7e3", "--text-secondary": "#a09d87", "--accent-blue": "#ccc58e", "--accent-green": "#8ecca8",
      "--accent-purple": "#b88ecc", "--accent-gold": "#ccba8e", "--accent-rose": "#cc8e99", "--border-color": "rgba(255, 255, 255, 0.06)",
      "--error-color": "#e0574a",
    },
  },
  {
    id: "light-simply-white", name: "Simply White", category: "standard", mode: "light", pairId: "simply-white",
    preview: { bg: "#f8f8f6", fg: "#282720", accent: "#716933", secondary: "#efefeb" },
    vars: {
      "--bg-primary": "#f8f8f6", "--bg-secondary": "#efefeb", "--bg-tertiary": "#e2e1da", "--bg-elevated": "#ffffff",
      "--text-primary": "#282720", "--text-secondary": "#706e5c", "--accent-blue": "#716933", "--accent-green": "#33714c",
      "--accent-purple": "#5c3371", "--accent-gold": "#715e33", "--accent-rose": "#71333d", "--border-color": "rgba(0, 0, 0, 0.08)",
      "--error-color": "#c2402f",
    },
  },
  // ── Pair: Baize (Bottle Green · Antique Brass · Oxblood) — the deep
  // green baize that lines a bespoke gun case, with brass fittings and an
  // oxblood leather trim: an English-country-house palette (gunroom green,
  // walnut, hide) rather than any single maker's mark — see the
  // "Remove brand-named themes" commit for why this stays generic. Hero
  // accent sits in the brass slot instead of true blue, same convention
  // Blaze/Marshal use for their off-hue heroes; the purple slot is
  // repurposed as a walnut-wood brown to keep the whole ramp inside the
  // green/brass/hide family.
  {
    id: "dark-baize", name: "Baize", category: "standard", mode: "dark", pairId: "baize",
    preview: { bg: "#14231c", fg: "#f2e9d8", accent: "#c9a24a", secondary: "#1c2f26" },
    vars: {
      "--bg-primary": "#14231c", "--bg-secondary": "#1c2f26", "--bg-tertiary": "#243c30", "--bg-elevated": "#2d4a3b",
      "--text-primary": "#f2e9d8", "--text-secondary": "#a3b39d", "--accent-blue": "#c9a24a", "--accent-green": "#4a7a5a",
      "--accent-purple": "#8a5a3e", "--accent-gold": "#ddbf74", "--accent-rose": "#8a3a3a",
      "--border-color": "rgba(201, 162, 74, 0.16)", "--error-color": "#c0392b",
      "--success-color": "#5a8f5a", "--warning-color": "#c9a24a", "--info-color": "#7f97a0",
      "--accent-color": "#c9a24a", "--glow-accent": "0 0 20px rgba(201, 162, 74, 0.22)",
    },
  },
  {
    id: "light-baize", name: "Baize", category: "standard", mode: "light", pairId: "baize",
    preview: { bg: "#f7f4e9", fg: "#1f2a20", accent: "#8a6a1f", secondary: "#eee7d4" },
    vars: {
      "--bg-primary": "#f7f4e9", "--bg-secondary": "#eee7d4", "--bg-tertiary": "#dcdcc0", "--bg-elevated": "#ffffff",
      "--text-primary": "#1f2a20", "--text-secondary": "#5c6b57", "--accent-blue": "#8a6a1f", "--accent-green": "#3c6b4a",
      "--accent-purple": "#6b4530", "--accent-gold": "#a3822e", "--accent-rose": "#7a3535",
      "--border-color": "rgba(138, 106, 31, 0.16)", "--error-color": "#b8503f",
      "--success-color": "#3c6b4a", "--warning-color": "#a3822e", "--info-color": "#55707a",
      "--accent-color": "#8a6a1f", "--glow-accent": "0 0 20px rgba(138, 106, 31, 0.15)",
    },
  },
];

/**
 * For each theme, ensure adequate contrast for buttons:
 * - --btn-primary-fg: text color for primary buttons (on accent-blue bg)
 * - --btn-error-fg: text color for error buttons (on error-color bg)
 * - --text-secondary must have >=3:1 contrast on --bg-tertiary
 *
 * This post-process step auto-corrects any theme missing these.
 */
function hexLum(hex: string): number {
  const h = hex.replace('#', '');
  const r = parseInt(h.substring(0, 2), 16) / 255;
  const g = parseInt(h.substring(2, 4), 16) / 255;
  const b = parseInt(h.substring(4, 6), 16) / 255;
  const srgb = (c: number) => c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
  return 0.2126 * srgb(r) + 0.7152 * srgb(g) + 0.0722 * srgb(b);
}
function contrast(h1: string, h2: string): number {
  const l1 = hexLum(h1), l2 = hexLum(h2);
  return (Math.max(l1, l2) + 0.05) / (Math.min(l1, l2) + 0.05);
}
function bestFgForBg(bg: string): string {
  return contrast("#ffffff", bg) >= contrast("#000000", bg) ? "#ffffff" : "#000000";
}

// Post-process: add --btn-primary-fg and --btn-error-fg
for (const t of THEMES) {
  const accent = t.vars["--accent-blue"];
  const err = t.vars["--error-color"];
  if (accent && !accent.startsWith("rgba")) {
    t.vars["--btn-primary-fg"] = bestFgForBg(accent);
  }
  if (err && !err.startsWith("rgba")) {
    t.vars["--btn-error-fg"] = bestFgForBg(err);
  }
  // Fix secondary text if contrast on bg-tertiary is < 3:1
  const sec = t.vars["--text-secondary"];
  const tert = t.vars["--bg-tertiary"];
  if (sec && tert && !sec.startsWith("rgba") && !tert.startsWith("rgba")) {
    if (contrast(sec, tert) < 3.0) {
      // Iteratively adjust until ≥3.1:1 contrast
      const h = sec.replace('#', '');
      let r = parseInt(h.substring(0, 2), 16);
      let g = parseInt(h.substring(2, 4), 16);
      let b = parseInt(h.substring(4, 6), 16);
      for (let i = 0; i < 60; i++) {
        if (t.mode === "dark") {
          r = Math.min(255, r + 3); g = Math.min(255, g + 3); b = Math.min(255, b + 3);
        } else {
          r = Math.max(0, r - 3); g = Math.max(0, g - 3); b = Math.max(0, b - 3);
        }
        const adj = `#${r.toString(16).padStart(2,'0')}${g.toString(16).padStart(2,'0')}${b.toString(16).padStart(2,'0')}`;
        if (contrast(adj, tert) >= 3.1) { t.vars["--text-secondary"] = adj; break; }
      }
    }
  }
  // Fix text-primary on bg-elevated if < 4.5:1
  const pri = t.vars["--text-primary"];
  const elev = t.vars["--bg-elevated"];
  if (pri && elev && !pri.startsWith("rgba") && !elev.startsWith("rgba")) {
    if (contrast(pri, elev) < 4.5) {
      const hp = pri.replace('#', '');
      let r = parseInt(hp.substring(0, 2), 16);
      let g = parseInt(hp.substring(2, 4), 16);
      let b = parseInt(hp.substring(4, 6), 16);
      for (let i = 0; i < 80; i++) {
        if (t.mode === "dark") {
          r = Math.min(255, r + 2); g = Math.min(255, g + 2); b = Math.min(255, b + 2);
        } else {
          r = Math.max(0, r - 2); g = Math.max(0, g - 2); b = Math.max(0, b - 2);
        }
        const adj = `#${r.toString(16).padStart(2,'0')}${g.toString(16).padStart(2,'0')}${b.toString(16).padStart(2,'0')}`;
        if (contrast(adj, elev) >= 4.5) { t.vars["--text-primary"] = adj; break; }
      }
    }
  }
}

/** Get the paired theme (dark↔light) for the given theme id */
export function getPairedTheme(currentId: string): ThemeDef | undefined {
  const current = THEMES.find(t => t.id === currentId);
  if (!current) return undefined;
  const targetMode = current.mode === "dark" ? "light" : "dark";
  return THEMES.find(t => t.pairId === current.pairId && t.mode === targetMode);
}

/** Union of every CSS var name any theme can set — used to clear stale inline vars on switch */
const ALL_THEME_VAR_KEYS: string[] = Array.from(
  new Set(THEMES.flatMap(t => Object.keys(t.vars)))
);

/* ── Color helpers (derive peripheral vars from the theme palette) ───── */

type ParsedColor = { r: number; g: number; b: number; a: number };

function parseColor(input: string): ParsedColor | null {
  const s = input.trim();
  if (s.startsWith("#")) {
    let h = s.slice(1);
    if (h.length === 3) h = h.split("").map(c => c + c).join("");
    if (h.length !== 6) return null;
    const r = parseInt(h.slice(0, 2), 16);
    const g = parseInt(h.slice(2, 4), 16);
    const b = parseInt(h.slice(4, 6), 16);
    if ([r, g, b].some(Number.isNaN)) return null;
    return { r, g, b, a: 1 };
  }
  const m = s.match(/^rgba?\(\s*([\d.]+)\s*,\s*([\d.]+)\s*,\s*([\d.]+)(?:\s*,\s*([\d.]+))?\s*\)$/i);
  if (m) return { r: +m[1], g: +m[2], b: +m[3], a: m[4] !== undefined ? +m[4] : 1 };
  return null;
}

function withAlpha(input: string, alpha: number): string {
  const c = parseColor(input);
  if (!c) return input;
  return `rgba(${Math.round(c.r)}, ${Math.round(c.g)}, ${Math.round(c.b)}, ${alpha})`;
}

function mixColor(a: string, b: string, ratio: number): string {
  const ca = parseColor(a);
  const cb = parseColor(b);
  if (!ca || !cb) return a;
  const r = Math.round(ca.r + (cb.r - ca.r) * ratio);
  const g = Math.round(ca.g + (cb.g - ca.g) * ratio);
  const blu = Math.round(ca.b + (cb.b - ca.b) * ratio);
  return `#${r.toString(16).padStart(2, "0")}${g.toString(16).padStart(2, "0")}${blu.toString(16).padStart(2, "0")}`;
}

/** Vars derived from the theme's core palette — cleared on switch to prevent bleed-through. */
const DERIVED_THEME_VAR_KEYS: string[] = [
  "--accent-color", "--accent-primary",
  "--success-color", "--warning-color", "--info-color",
  "--text-color", "--text-success", "--text-danger", "--text-info", "--text-warning", "--text-accent",
  "--text-tertiary", "--text-muted",
  "--border-subtle",
  "--success-bg", "--error-bg", "--warning-bg", "--info-bg", "--accent-bg",
  "--input-bg", "--bg-input", "--input-border", "--bg-hover",
  "--git-modified", "--git-added", "--git-deleted", "--git-ignored", "--git-conflicted",
  "--glass-bg", "--glow-accent",
];

/** Set every derived var that the theme didn't explicitly override. */
function applyDerivedVars(root: HTMLElement, theme: ThemeDef): void {
  const v = theme.vars;
  const set = (key: string, value: string) => {
    if (v[key] === undefined) root.style.setProperty(key, value);
  };
  const accentBlue = v["--accent-blue"]!;
  const accentGreen = v["--accent-green"]!;
  const accentGold = v["--accent-gold"]!;
  const accentRose = v["--accent-rose"]!;
  const errorColor = v["--error-color"]!;
  const textPrimary = v["--text-primary"]!;
  const textSecondary = v["--text-secondary"]!;
  const borderColor = v["--border-color"]!;
  const bgPrimary = v["--bg-primary"]!;
  const bgSecondary = v["--bg-secondary"]!;
  const bgTertiary = v["--bg-tertiary"]!;
  // Signature = the theme's "look at me" color shown in the preview swatch.
  // Drives primary UI accents (status bar, focus rings, active section text, etc.)
  // so the UI matches the swatch a user sees in Settings → Appearance.
  const signature = theme.preview.accent;

  set("--accent-color", signature);
  set("--accent-primary", signature);
  set("--success-color", accentGreen);
  set("--warning-color", accentGold);
  set("--info-color", signature);
  set("--text-color", textPrimary);
  set("--text-success", accentGreen);
  set("--text-danger", errorColor);
  set("--text-info", signature);
  set("--text-warning", accentGold);
  set("--text-accent", signature);

  set("--text-tertiary", mixColor(textPrimary, textSecondary, 0.5));
  set("--text-muted", mixColor(textSecondary, bgPrimary, 0.45));

  const borderAlpha = parseColor(borderColor)?.a ?? 0.06;
  set("--border-subtle", withAlpha(borderColor, borderAlpha / 2));

  set("--success-bg", withAlpha(accentGreen, 0.10));
  set("--error-bg", withAlpha(errorColor, 0.10));
  set("--warning-bg", withAlpha(accentGold, 0.10));
  set("--info-bg", withAlpha(accentBlue, 0.10));
  set("--accent-bg", withAlpha(signature, 0.15));

  const inputBg = theme.mode === "light" ? "#ffffff" : bgPrimary;
  set("--input-bg", inputBg);
  set("--bg-input", inputBg);
  set("--input-border", borderColor);
  set("--bg-hover", bgTertiary);

  set("--git-modified", accentGold);
  set("--git-added", accentGreen);
  set("--git-deleted", errorColor);
  set("--git-ignored", mixColor(textSecondary, bgPrimary, 0.5));
  set("--git-conflicted", accentRose);

  set("--glass-bg", withAlpha(bgSecondary, theme.mode === "light" ? 0.72 : 0.75));
  set("--glow-accent", `0 0 20px ${withAlpha(signature, 0.15)}`);
}

/** Apply a theme by id — sets CSS vars, localStorage, and notifies Monaco editor */
export function applyThemeById(themeId: string): void {
  const theme = THEMES.find(t => t.id === themeId);
  if (!theme) return;
  const root = document.documentElement;
  // Clear every var any theme could have set so nothing from a previous theme bleeds through
  ALL_THEME_VAR_KEYS.forEach(key => root.style.removeProperty(key));
  DERIVED_THEME_VAR_KEYS.forEach(key => root.style.removeProperty(key));
  try { localStorage.setItem("odoo-rs-theme-id", theme.id); localStorage.setItem("odoo-rs-theme", theme.mode); } catch { /* storage unavailable */ }
  root.setAttribute("data-theme", theme.mode);
  for (const [key, value] of Object.entries(theme.vars)) {
    root.style.setProperty(key, value);
  }
  applyDerivedVars(root, theme);
  // cache the applied variables so the boot script can paint the saved theme before React hydrates (no flash)
  try { localStorage.setItem("odoo-rs-theme-css", root.style.cssText); } catch { /* storage unavailable */ }
  window.dispatchEvent(new CustomEvent("odoo-rs-theme-change", { detail: { themeId: theme.id, mode: theme.mode } }));
}
