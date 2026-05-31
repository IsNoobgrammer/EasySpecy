import { create } from "zustand";
import { persist } from "zustand/middleware";

export type Theme = "dark" | "light";

interface ThemeState {
  theme: Theme;
  setTheme: (theme: Theme) => void;
  toggleTheme: () => void;
}

export const useThemeStore = create<ThemeState>()(
  persist(
    (set, get) => ({
      theme: "dark",
      setTheme: (theme) => {
        document.documentElement.setAttribute("data-theme", theme);
        set({ theme });
      },
      toggleTheme: () => {
        const next = get().theme === "dark" ? "light" : "dark";
        document.documentElement.setAttribute("data-theme", next);
        set({ theme: next });
      },
    }),
    {
      name: "easyspecy-theme",
      onRehydrateStorage: () => (state) => {
        if (state) {
          document.documentElement.setAttribute("data-theme", state.theme);
        }
      },
    }
  )
);

// Initialize theme on load
export function initTheme() {
  const stored = localStorage.getItem("easyspecy-theme");
  if (stored) {
    try {
      const parsed = JSON.parse(stored);
      document.documentElement.setAttribute("data-theme", parsed.state?.theme || "dark");
    } catch {
      document.documentElement.setAttribute("data-theme", "dark");
    }
  } else {
    document.documentElement.setAttribute("data-theme", "dark");
  }
}
