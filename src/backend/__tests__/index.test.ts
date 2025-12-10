import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

// Store original values
const originalWindow = globalThis.window;
let originalEnv: Record<string, unknown>;

describe("backend/index", () => {
  beforeEach(() => {
    vi.resetModules();
    // Store original import.meta.env
    originalEnv = { ...import.meta.env };
  });

  afterEach(() => {
    // Restore window
    globalThis.window = originalWindow;
    // Restore env
    Object.assign(import.meta.env, originalEnv);
    vi.restoreAllMocks();
  });

  describe("isTauri", () => {
    it("returns true when window.__TAURI__ exists", async () => {
      globalThis.window = { __TAURI__: {} } as unknown as Window & typeof globalThis;
      const { isTauri } = await import("../index");
      expect(isTauri()).toBe(true);
    });

    it("returns false when window.__TAURI__ is undefined", async () => {
      globalThis.window = {} as Window & typeof globalThis;
      const { isTauri } = await import("../index");
      expect(isTauri()).toBe(false);
    });

    it("returns false when window is undefined (SSR)", async () => {
      // @ts-expect-error - intentionally setting to undefined for SSR test
      globalThis.window = undefined;
      const { isTauri } = await import("../index");
      expect(isTauri()).toBe(false);
    });
  });

  describe("isServerMode", () => {
    it("returns true when VITE_BACKEND_MODE is 'http'", async () => {
      globalThis.window = { __TAURI__: {} } as unknown as Window & typeof globalThis;
      import.meta.env.VITE_BACKEND_MODE = "http";
      const { isServerMode } = await import("../index");
      expect(isServerMode()).toBe(true);
    });

    it("returns false when VITE_BACKEND_MODE is 'tauri'", async () => {
      globalThis.window = {} as Window & typeof globalThis;
      import.meta.env.VITE_BACKEND_MODE = "tauri";
      const { isServerMode } = await import("../index");
      expect(isServerMode()).toBe(false);
    });

    it("auto-detects server mode when not in Tauri", async () => {
      globalThis.window = {} as Window & typeof globalThis;
      delete import.meta.env.VITE_BACKEND_MODE;
      const { isServerMode } = await import("../index");
      expect(isServerMode()).toBe(true);
    });

    it("auto-detects Tauri mode when __TAURI__ exists", async () => {
      globalThis.window = { __TAURI__: {} } as unknown as Window & typeof globalThis;
      delete import.meta.env.VITE_BACKEND_MODE;
      const { isServerMode } = await import("../index");
      expect(isServerMode()).toBe(false);
    });
  });

  describe("getBackendClient", () => {
    it("returns httpClient in server mode", async () => {
      globalThis.window = {} as Window & typeof globalThis;
      import.meta.env.VITE_BACKEND_MODE = "http";
      const { getBackendClient, httpClient } = await import("../index");
      expect(getBackendClient()).toBe(httpClient);
    });

    it("returns tauriClient in tauri mode", async () => {
      globalThis.window = { __TAURI__: {} } as unknown as Window & typeof globalThis;
      import.meta.env.VITE_BACKEND_MODE = "tauri";
      const { getBackendClient, tauriClient } = await import("../index");
      expect(getBackendClient()).toBe(tauriClient);
    });
  });

  describe("exports", () => {
    it("re-exports types from ./types", async () => {
      const index = await import("../index");
      expect(typeof index.isBackendError).toBe("function");
    });

    it("exports tauriClient", async () => {
      const { tauriClient } = await import("../index");
      expect(tauriClient).toBeDefined();
      expect(typeof tauriClient.getProxyStatus).toBe("function");
    });

    it("exports httpClient", async () => {
      const { httpClient } = await import("../index");
      expect(httpClient).toBeDefined();
      expect(typeof httpClient.getProxyStatus).toBe("function");
    });
  });
});
