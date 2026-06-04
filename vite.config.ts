import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(() => ({
  plugins: [react(), tailwindcss()],

  // Vite options tailored for Tauri development
  clearScreen: false,

  // Expose Tauri IPC to overlay windows
  build: {
    target: "esnext",
    minify: "esbuild" as const,
    rollupOptions: {
      output: {
        manualChunks: {
          "vendor-motion": ["motion"],
          "vendor-ui": ["lucide-react", "react-rnd"],
          "vendor-tauri": [
            "@tauri-apps/api",
            "@tauri-apps/plugin-opener",
            "@tauri-apps/plugin-global-shortcut",
            "@tauri-apps/plugin-process",
            "@tauri-apps/plugin-updater",
          ],
        },
      },
    },
  },
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
}));
