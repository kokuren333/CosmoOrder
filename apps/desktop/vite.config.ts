import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// The desktop bundle is a local file served by Tauri's own asset protocol.
// It has no server, no proxy and no runtime dependency on the network.
export default defineConfig({
  plugins: [react()],
  base: "./",
  clearScreen: false,
  server: {
    host: "127.0.0.1",
    // A dedicated port: the shell never points at a dev server, so a plain
    // `vite` run is only a convenience while editing the renderer.
    port: 5273,
    strictPort: true,
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
    target: "chrome120",
    sourcemap: false,
    assetsDir: "assets",
  },
});
