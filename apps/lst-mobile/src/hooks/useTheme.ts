import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { commands, type ThemeData } from "../bindings";

export type ThemePayload = {
  vars?: Record<string, string>;
};

export function useTheme() {
  const [themeData, setThemeData] = useState<ThemeData | null>(null);
  const [availableThemes, setAvailableThemes] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);

  const loadCurrentTheme = async () => {
    try {
      const result = await commands.getCurrentTheme();
      if (result.status === "ok") {
        setThemeData(result.data);
        applyThemeToDOM(result.data);
      }
    } catch (error) {
      console.error("Failed to load current theme:", error);
    }
  };

  const loadAvailableThemes = async () => {
    try {
      const result = await commands.listThemes();
      if (result.status === "ok") {
        setAvailableThemes(result.data);
      }
    } catch (error) {
      console.error("Failed to load available themes:", error);
    }
  };

  const applyTheme = async (themeName: string) => {
    try {
      const result = await commands.applyTheme(themeName);
      if (result.status === "ok") {
        setThemeData(result.data);
        applyThemeToDOM(result.data);
      }
    } catch (error) {
      console.error("Failed to apply theme:", error);
    }
  };

  useEffect(() => {
    const initializeTheme = async () => {
      setLoading(true);
      await Promise.all([loadCurrentTheme(), loadAvailableThemes()]);
      setLoading(false);
    };

    initializeTheme();

    const unlisten = listen<ThemeData>("theme-update", ({ payload }) => {
      if (payload) {
        setThemeData(payload);
        applyThemeToDOM(payload);
      }
    });

    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  return {
    themeData,
    availableThemes,
    loading,
    applyTheme,
    reload: loadCurrentTheme,
  };
}

function applyThemeToDOM(themeData: ThemeData) {
  const root = document.documentElement;
  
  // Parse and apply CSS variables
  const cssVariables = themeData.css_variables;
  const lines = cssVariables.split("\n");
  
  for (const line of lines) {
    const trimmed = line.trim();
    if (trimmed.startsWith("--") && trimmed.includes(":")) {
      const colonIndex = trimmed.indexOf(":");
      const property = trimmed.substring(0, colonIndex).trim();
      const value = trimmed.substring(colonIndex + 1).replace(";", "").trim();
      root.style.setProperty(property, value);
    }
  }
  
  // Apply scheme class
  root.classList.remove("light", "dark");
  root.classList.add(themeData.scheme);
}
