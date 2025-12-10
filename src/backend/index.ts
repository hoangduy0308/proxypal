/**
 * ProxyPal Backend Adapter Entry Point
 *
 * Provides environment detection and automatic client selection between
 * Tauri (desktop) and HTTP (server) modes.
 */

// Re-export all types
export * from "./types";

import type { BackendClient } from "./types";
import { tauriClient } from "./tauriClient";
import { httpClient } from "./httpClient";

/**
 * Check if running in Tauri desktop environment.
 * Handles SSR/Node environments where window is undefined.
 */
export function isTauri(): boolean {
  return (
    typeof window !== "undefined" &&
    "__TAURI__" in window &&
    window.__TAURI__ !== undefined
  );
}

/**
 * Check if running in server/HTTP mode.
 * Priority: env var override > auto-detect based on Tauri presence.
 */
export function isServerMode(): boolean {
  // Explicit override via env var
  if (import.meta.env.VITE_BACKEND_MODE === "http") return true;
  if (import.meta.env.VITE_BACKEND_MODE === "tauri") return false;
  // Auto-detect: not Tauri = server mode
  return !isTauri();
}

/**
 * Get the appropriate backend client based on environment.
 * Use this when you need a fresh reference based on current environment.
 */
export function getBackendClient(): BackendClient {
  return isServerMode() ? httpClient : tauriClient;
}

/**
 * Default backend client instance.
 * Note: This is evaluated at module load time.
 * Use getBackendClient() if environment may change at runtime.
 */
export const backendClient: BackendClient = isServerMode()
  ? httpClient
  : tauriClient;

// Re-export individual clients for testing and direct access
export { tauriClient } from "./tauriClient";
export { httpClient } from "./httpClient";
