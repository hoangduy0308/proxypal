import { defineConfig } from "vite";
import solid from "vite-plugin-solid";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;
// @ts-expect-error process is a nodejs global
const isProduction = process.env.NODE_ENV === "production";

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [solid()],

  // Allow both Tauri and HTTP env vars
  envPrefix: ["VITE_", "TAURI_"],

  // Base path for assets
  base: "/",

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  clearScreen: false,

  build: {
    target: "esnext",
    outDir: "dist",
    // Enable minification for production
    minify: "esbuild" as const,
    // Generate source maps for debugging (disabled in production)
    sourcemap: !isProduction,
    rollupOptions: {
      output: {
        manualChunks: {
          // Optimize chunk splitting
          solid: ["solid-js"],
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
      // Tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
    // Proxy API requests to backend server during development
    proxy: {
      "/api": {
        target: "http://localhost:3000",
        changeOrigin: true,
      },
      "/oauth": {
        target: "http://localhost:3000",
        changeOrigin: true,
      },
    },
  },
}));
