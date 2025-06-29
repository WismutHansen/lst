import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";

export type ThemePayload = {
  vars?: Record<string, string>;
};

export function useTheme() {
  useEffect(() => {
    const unlisten = listen<ThemePayload>("theme-update", ({ payload }) => {
      if (payload?.vars) {
        const root = document.documentElement;
        Object.entries(payload.vars).forEach(([k, v]) => {
          root.style.setProperty(`--${k}`, v);
        });
      }
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, []);
}
