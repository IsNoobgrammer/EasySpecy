import { useState, useEffect, useCallback, useRef } from "react";
import { motion, AnimatePresence } from "motion/react";
import { invoke } from "@tauri-apps/api/core";
import { Icon } from "./Icon";
import { useStore } from "../stores/recording";
import { useThemeStore } from "../lib/theme";

// ═══ TYPES ═══

export interface ContextMenuItem {
  id: string;
  label: string;
  icon?: string;
  shortcut?: string;
  danger?: boolean;
  disabled?: boolean;
  separator?: boolean;
  onClick: () => void;
}

interface ContextMenuState {
  x: number;
  y: number;
  items: ContextMenuItem[];
  visible: boolean;
}

// ═══ GLOBAL CONTEXT MENU STORE ═══

type ContextMenuHandler = (e: React.MouseEvent, items: ContextMenuItem[]) => void;

let _showContextMenu: ContextMenuHandler | null = null;

/** Call this from any component to show a context menu at the mouse position */
export function showContextMenu(e: React.MouseEvent | MouseEvent, items: ContextMenuItem[]) {
  e.preventDefault();
  e.stopPropagation();
  if (_showContextMenu) {
    _showContextMenu(e as React.MouseEvent, items);
  }
}

// ═══ CONTEXT MENU PROVIDER ═══

export function ContextMenuProvider({ children }: { children: React.ReactNode }) {
  const [state, setState] = useState<ContextMenuState>({
    x: 0, y: 0, items: [], visible: false,
  });
  const menuRef = useRef<HTMLDivElement>(null);

  // Register global handler
  useEffect(() => {
    _showContextMenu = (_e, items) => {
      // Clamp position to viewport
      const x = Math.min(_e.clientX, window.innerWidth - 220);
      const y = Math.min(_e.clientY, window.innerHeight - items.length * 36 - 16);
      setState({ x, y, items, visible: true });
    };
    return () => { _showContextMenu = null; };
  }, []);

  // Close on click outside
  const close = useCallback(() => {
    setState((s) => ({ ...s, visible: false }));
  }, []);

  useEffect(() => {
    if (!state.visible) return;
    const handleClick = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        close();
      }
    };
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
    };
    // Delay to avoid catching the opening click
    setTimeout(() => {
      document.addEventListener("mousedown", handleClick);
      document.addEventListener("keydown", handleKey);
    }, 10);
    return () => {
      document.removeEventListener("mousedown", handleClick);
      document.removeEventListener("keydown", handleKey);
    };
  }, [state.visible, close]);

  return (
    <>
      {children}
      <AnimatePresence>
        {state.visible && (
          <motion.div
            ref={menuRef}
            initial={{ opacity: 0, scale: 0.92, y: -4 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.92, y: -4 }}
            transition={{ duration: 0.12, ease: [0.16, 1, 0.3, 1] }}
            className="fixed z-[10000] py-1 min-w-[180px] max-w-[240px]"
            style={{
              left: state.x,
              top: state.y,
              background: "var(--bg-surface)",
              border: "var(--border-thin) solid var(--border-default)",
              borderRadius: "var(--radius-md)",
              boxShadow: "var(--shadow-lg)",
              backdropFilter: "blur(12px)",
            }}
          >
            {state.items.map((item, i) => {
              if (item.separator) {
                return (
                  <div
                    key={`sep-${i}`}
                    className="my-1 mx-2"
                    style={{ borderTop: "var(--border-thin) solid var(--border-default)" }}
                  />
                );
              }
              return (
                <motion.button
                  key={item.id}
                  onClick={() => {
                    if (!item.disabled) {
                      item.onClick();
                      close();
                    }
                  }}
                  className="w-full flex items-center gap-2.5 px-3 py-1.5 cursor-pointer text-left"
                  style={{
                    background: "transparent",
                    border: "none",
                    color: item.disabled
                      ? "var(--text-muted)"
                      : item.danger
                        ? "var(--accent-danger)"
                        : "var(--text-primary)",
                    opacity: item.disabled ? 0.5 : 1,
                    fontFamily: "var(--font-body)",
                    fontSize: "0.75rem",
                  }}
                  whileHover={
                    item.disabled
                      ? {}
                      : {
                          background: item.danger
                            ? "oklch(from var(--accent-danger) l c h / 0.1)"
                            : "var(--surface-container-high)",
                        }
                  }
                >
                  {item.icon && (
                    <Icon
                      name={item.icon}
                      size={14}
                      style={{
                        color: item.danger ? "var(--accent-danger)" : "var(--text-muted)",
                      }}
                    />
                  )}
                  <span className="flex-1">{item.label}</span>
                  {item.shortcut && (
                    <span
                      className="font-mono"
                      style={{ fontSize: "0.6rem", color: "var(--text-muted)", letterSpacing: "0.04em" }}
                    >
                      {item.shortcut}
                    </span>
                  )}
                </motion.button>
              );
            })}
          </motion.div>
        )}
      </AnimatePresence>
    </>
  );
}

// ═══ CONVENIENCE HOOKS ═══

/** Build context menu items for a recording entry */
export function useRecordingContextMenu() {
  const openPath = useStore((s) => s.openPath);
  const revealInExplorer = useStore((s) => s.revealInExplorer);
  const copyToClipboard = useStore((s) => s.copyToClipboard);
  const loadHistory = useStore((s) => s.loadHistory);
  const addToast = useStore((s) => s.addToast);

  return {
    onContextMenu: (e: React.MouseEvent, entry: { output_path: string; id: string }) => {
      showContextMenu(e, [
        {
          id: "open",
          label: "Open",
          icon: "play_arrow",
          onClick: () => openPath(entry.output_path),
        },
        {
          id: "reveal",
          label: "Reveal in Explorer",
          icon: "folder_open",
          onClick: () => revealInExplorer(entry.output_path),
        },
        {
          id: "copy-path",
          label: "Copy Path",
          icon: "content_copy",
          shortcut: "Ctrl+C",
          onClick: () => copyToClipboard(entry.output_path),
        },
        { id: "sep-1", label: "", separator: true, onClick: () => {} },
        {
          id: "delete",
          label: "Delete",
          icon: "delete",
          danger: true,
          onClick: async () => {
            try {
              await invoke("delete_recording", { path: entry.output_path });
              loadHistory();
              addToast("Recording deleted", "info");
            } catch (err) {
              addToast("Failed to delete", "error");
            }
          },
        },
      ]);
    },
  };
}

/** Build context menu items for general app area */
export function useAppContextMenu(navigate: (page: "dashboard" | "settings") => void) {
  return {
    onContextMenu: (e: React.MouseEvent) => {
      showContextMenu(e, [
        {
          id: "settings",
          label: "Settings",
          icon: "settings",
          onClick: () => navigate("settings"),
        },

        { id: "sep-1", label: "", separator: true, onClick: () => {} },
        {
          id: "theme",
          label: "Toggle Theme",
          icon: "brightness_6",
          onClick: () => useThemeStore.getState().toggleTheme(),
        },
        { id: "sep-2", label: "", separator: true, onClick: () => {} },
        {
          id: "quit",
          label: "Quit EasySpecy",
          icon: "close",
          shortcut: "Alt+F4",
          danger: true,
          onClick: () => {
            window.close();
          },
        },
      ]);
    },
  };
}
