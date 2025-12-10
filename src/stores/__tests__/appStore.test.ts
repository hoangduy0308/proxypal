import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

// Mock the backend module
vi.mock("../../backend", () => ({
  backendClient: {
    getProxyStatus: vi.fn(),
    getConfig: vi.fn(),
    startProxy: vi.fn(),
    stopProxy: vi.fn(),
    getUsageStats: vi.fn(),
  },
  isTauri: vi.fn(),
  isServerMode: vi.fn(),
}));

// Mock the tauri module
vi.mock("../../lib/tauri", () => ({
  getAuthStatus: vi.fn(),
  refreshAuthStatus: vi.fn(),
  completeOAuth: vi.fn(),
  onProxyStatusChanged: vi.fn(),
  onAuthStatusChanged: vi.fn(),
  onOAuthCallback: vi.fn(),
  onTrayToggleProxy: vi.fn(),
  showSystemNotification: vi.fn(),
}));

import { backendClient, isTauri } from "../../backend";
import {
  onProxyStatusChanged,
  onAuthStatusChanged,
  onOAuthCallback,
  onTrayToggleProxy,
  refreshAuthStatus,
} from "../../lib/tauri";

describe("appStore", () => {
  const mockUnlisten = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();

    // Default mock implementations
    vi.mocked(backendClient.getProxyStatus).mockResolvedValue({
      running: false,
      port: 8317,
      endpoint: "http://localhost:8317/v1",
    });

    vi.mocked(backendClient.getConfig).mockResolvedValue({
      proxyPort: 8317,
      autoStartProxy: false,
    });

    vi.mocked(refreshAuthStatus).mockResolvedValue({
      claude: 0,
      openai: 0,
      gemini: 0,
      qwen: 0,
      iflow: 0,
      vertex: 0,
      antigravity: 0,
    });

    vi.mocked(onProxyStatusChanged).mockResolvedValue(mockUnlisten);
    vi.mocked(onAuthStatusChanged).mockResolvedValue(mockUnlisten);
    vi.mocked(onOAuthCallback).mockResolvedValue(mockUnlisten);
    vi.mocked(onTrayToggleProxy).mockResolvedValue(mockUnlisten);
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe("initialization", () => {
    it("should load proxy status and config from backend adapter", async () => {
      vi.mocked(isTauri).mockReturnValue(true);

      const { appStore } = await import("../app");

      await appStore.initialize();

      expect(backendClient.getProxyStatus).toHaveBeenCalled();
      expect(backendClient.getConfig).toHaveBeenCalled();
      expect(appStore.isInitialized()).toBe(true);
    });

    it("should update proxy status from backend response", async () => {
      vi.mocked(isTauri).mockReturnValue(true);
      vi.mocked(backendClient.getProxyStatus).mockResolvedValue({
        running: true,
        port: 9000,
        endpoint: "http://localhost:9000/v1",
      });

      // Re-import to get fresh store
      vi.resetModules();
      vi.doMock("../../backend", () => ({
        backendClient: {
          getProxyStatus: vi.fn().mockResolvedValue({
            running: true,
            port: 9000,
            endpoint: "http://localhost:9000/v1",
          }),
          getConfig: vi.fn().mockResolvedValue({
            proxyPort: 9000,
            autoStartProxy: false,
          }),
          startProxy: vi.fn(),
          stopProxy: vi.fn(),
        },
        isTauri: vi.fn().mockReturnValue(true),
        isServerMode: vi.fn().mockReturnValue(false),
      }));
      vi.doMock("../../lib/tauri", () => ({
        getAuthStatus: vi.fn(),
        refreshAuthStatus: vi.fn().mockResolvedValue({
          claude: 0, openai: 0, gemini: 0, qwen: 0, iflow: 0, vertex: 0, antigravity: 0,
        }),
        completeOAuth: vi.fn(),
        onProxyStatusChanged: vi.fn().mockResolvedValue(mockUnlisten),
        onAuthStatusChanged: vi.fn().mockResolvedValue(mockUnlisten),
        onOAuthCallback: vi.fn().mockResolvedValue(mockUnlisten),
        onTrayToggleProxy: vi.fn().mockResolvedValue(mockUnlisten),
        showSystemNotification: vi.fn(),
      }));

      const { appStore: freshStore } = await import("../app");
      await freshStore.initialize();

      expect(freshStore.proxyStatus().running).toBe(true);
      expect(freshStore.proxyStatus().port).toBe(9000);
    });
  });

  describe("Tauri mode", () => {
    it("should register event listeners in Tauri mode", async () => {
      vi.resetModules();
      const mockOnProxyStatusChanged = vi.fn().mockResolvedValue(mockUnlisten);
      const mockOnAuthStatusChanged = vi.fn().mockResolvedValue(mockUnlisten);
      const mockOnOAuthCallback = vi.fn().mockResolvedValue(mockUnlisten);
      const mockOnTrayToggleProxy = vi.fn().mockResolvedValue(mockUnlisten);

      vi.doMock("../../backend", () => ({
        backendClient: {
          getProxyStatus: vi.fn().mockResolvedValue({
            running: false,
            port: 8317,
            endpoint: "http://localhost:8317/v1",
          }),
          getConfig: vi.fn().mockResolvedValue({
            proxyPort: 8317,
            autoStartProxy: false,
          }),
          startProxy: vi.fn(),
          stopProxy: vi.fn(),
        },
        isTauri: vi.fn().mockReturnValue(true),
        isServerMode: vi.fn().mockReturnValue(false),
      }));
      vi.doMock("../../lib/tauri", () => ({
        getAuthStatus: vi.fn(),
        refreshAuthStatus: vi.fn().mockResolvedValue({
          claude: 0, openai: 0, gemini: 0, qwen: 0, iflow: 0, vertex: 0, antigravity: 0,
        }),
        completeOAuth: vi.fn(),
        onProxyStatusChanged: mockOnProxyStatusChanged,
        onAuthStatusChanged: mockOnAuthStatusChanged,
        onOAuthCallback: mockOnOAuthCallback,
        onTrayToggleProxy: mockOnTrayToggleProxy,
        showSystemNotification: vi.fn(),
      }));

      const { appStore: tauriStore } = await import("../app");
      await tauriStore.initialize();

      expect(mockOnProxyStatusChanged).toHaveBeenCalled();
      expect(mockOnAuthStatusChanged).toHaveBeenCalled();
      expect(mockOnOAuthCallback).toHaveBeenCalled();
      expect(mockOnTrayToggleProxy).toHaveBeenCalled();
    });
  });

  describe("HTTP mode", () => {
    it("should NOT register Tauri event listeners in HTTP mode", async () => {
      vi.resetModules();
      const mockOnProxyStatusChanged = vi.fn().mockResolvedValue(mockUnlisten);
      const mockOnAuthStatusChanged = vi.fn().mockResolvedValue(mockUnlisten);
      const mockOnOAuthCallback = vi.fn().mockResolvedValue(mockUnlisten);
      const mockOnTrayToggleProxy = vi.fn().mockResolvedValue(mockUnlisten);

      vi.doMock("../../backend", () => ({
        backendClient: {
          getProxyStatus: vi.fn().mockResolvedValue({
            running: true,
            port: 8317,
            endpoint: "http://localhost:8317/v1",
          }),
          getConfig: vi.fn().mockResolvedValue({
            proxyPort: 8317,
            autoStartProxy: false,
          }),
          startProxy: vi.fn(),
          stopProxy: vi.fn(),
        },
        isTauri: vi.fn().mockReturnValue(false),
        isServerMode: vi.fn().mockReturnValue(true),
      }));
      vi.doMock("../../lib/tauri", () => ({
        getAuthStatus: vi.fn(),
        refreshAuthStatus: vi.fn(),
        completeOAuth: vi.fn(),
        onProxyStatusChanged: mockOnProxyStatusChanged,
        onAuthStatusChanged: mockOnAuthStatusChanged,
        onOAuthCallback: mockOnOAuthCallback,
        onTrayToggleProxy: mockOnTrayToggleProxy,
        showSystemNotification: vi.fn(),
      }));

      const { appStore: httpStore } = await import("../app");
      await httpStore.initialize();

      expect(mockOnProxyStatusChanged).not.toHaveBeenCalled();
      expect(mockOnAuthStatusChanged).not.toHaveBeenCalled();
      expect(mockOnOAuthCallback).not.toHaveBeenCalled();
      expect(mockOnTrayToggleProxy).not.toHaveBeenCalled();
    });

    it("should navigate to dashboard in HTTP mode", async () => {
      vi.resetModules();
      vi.doMock("../../backend", () => ({
        backendClient: {
          getProxyStatus: vi.fn().mockResolvedValue({
            running: true,
            port: 8317,
            endpoint: "http://localhost:8317/v1",
          }),
          getConfig: vi.fn().mockResolvedValue({
            proxyPort: 8317,
            autoStartProxy: false,
          }),
          startProxy: vi.fn(),
          stopProxy: vi.fn(),
        },
        isTauri: vi.fn().mockReturnValue(false),
        isServerMode: vi.fn().mockReturnValue(true),
      }));
      vi.doMock("../../lib/tauri", () => ({
        getAuthStatus: vi.fn(),
        refreshAuthStatus: vi.fn(),
        completeOAuth: vi.fn(),
        onProxyStatusChanged: vi.fn().mockResolvedValue(mockUnlisten),
        onAuthStatusChanged: vi.fn().mockResolvedValue(mockUnlisten),
        onOAuthCallback: vi.fn().mockResolvedValue(mockUnlisten),
        onTrayToggleProxy: vi.fn().mockResolvedValue(mockUnlisten),
        showSystemNotification: vi.fn(),
      }));

      const { appStore: httpStore } = await import("../app");
      await httpStore.initialize();

      expect(httpStore.currentPage()).toBe("dashboard");
    });
  });

  describe("navigation", () => {
    it("should support 'users' page type", async () => {
      vi.mocked(isTauri).mockReturnValue(true);
      const { appStore } = await import("../app");

      appStore.setCurrentPage("users");
      expect(appStore.currentPage()).toBe("users");
    });

    it("should support 'usage' page type", async () => {
      vi.mocked(isTauri).mockReturnValue(true);
      const { appStore } = await import("../app");

      appStore.setCurrentPage("usage");
      expect(appStore.currentPage()).toBe("usage");
    });

    it("should support all page types", async () => {
      vi.mocked(isTauri).mockReturnValue(true);
      const { appStore } = await import("../app");

      const pageTypes = [
        "welcome",
        "dashboard",
        "settings",
        "api-keys",
        "auth-files",
        "logs",
        "analytics",
        "users",
        "usage",
      ] as const;

      for (const page of pageTypes) {
        appStore.setCurrentPage(page);
        expect(appStore.currentPage()).toBe(page);
      }
    });
  });
});
