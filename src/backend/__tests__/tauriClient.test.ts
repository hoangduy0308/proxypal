import { describe, it, expect, vi, beforeEach } from "vitest";
import type { BackendError } from "../types";
import { isBackendError } from "../types";

vi.mock("../../lib/tauri", () => ({
  startProxy: vi.fn(),
  stopProxy: vi.fn(),
  getProxyStatus: vi.fn(),
  getConfig: vi.fn(),
  saveConfig: vi.fn(),
  getUsageStats: vi.fn(),
  getAuthStatus: vi.fn(),
  openOAuth: vi.fn(),
  disconnectProvider: vi.fn(),
  onProxyStatusChanged: vi.fn(),
  onAuthStatusChanged: vi.fn(),
  onRequestLog: vi.fn(),
}));

import { tauriClient } from "../tauriClient";
import * as tauri from "../../lib/tauri";

const mockTauri = tauri as unknown as {
  startProxy: ReturnType<typeof vi.fn>;
  stopProxy: ReturnType<typeof vi.fn>;
  getProxyStatus: ReturnType<typeof vi.fn>;
  getConfig: ReturnType<typeof vi.fn>;
  saveConfig: ReturnType<typeof vi.fn>;
  getUsageStats: ReturnType<typeof vi.fn>;
  getAuthStatus: ReturnType<typeof vi.fn>;
  openOAuth: ReturnType<typeof vi.fn>;
  disconnectProvider: ReturnType<typeof vi.fn>;
  onProxyStatusChanged: ReturnType<typeof vi.fn>;
  onAuthStatusChanged: ReturnType<typeof vi.fn>;
  onRequestLog: ReturnType<typeof vi.fn>;
};

describe("TauriBackendClient", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("Proxy Management", () => {
    it("getProxyStatus delegates to Tauri and maps response", async () => {
      mockTauri.getProxyStatus.mockResolvedValue({
        running: true,
        port: 9999,
        endpoint: "http://localhost:9999",
      });

      const result = await tauriClient.getProxyStatus();

      expect(mockTauri.getProxyStatus).toHaveBeenCalled();
      expect(result).toEqual({
        running: true,
        port: 9999,
        endpoint: "http://localhost:9999",
      });
    });

    it("startProxy delegates to Tauri and maps response", async () => {
      mockTauri.startProxy.mockResolvedValue({
        running: true,
        port: 8080,
        endpoint: "http://localhost:8080",
      });

      const result = await tauriClient.startProxy();

      expect(mockTauri.startProxy).toHaveBeenCalled();
      expect(result.running).toBe(true);
      expect(result.port).toBe(8080);
    });

    it("stopProxy delegates to Tauri and maps response", async () => {
      mockTauri.stopProxy.mockResolvedValue({
        running: false,
        port: 8080,
        endpoint: "http://localhost:8080",
      });

      const result = await tauriClient.stopProxy();

      expect(mockTauri.stopProxy).toHaveBeenCalled();
      expect(result.running).toBe(false);
    });

    it("restartProxy stops then starts proxy", async () => {
      mockTauri.stopProxy.mockResolvedValue({ running: false, port: 8080, endpoint: "" });
      mockTauri.startProxy.mockResolvedValue({
        running: true,
        port: 8080,
        endpoint: "http://localhost:8080",
      });

      const result = await tauriClient.restartProxy();

      expect(mockTauri.stopProxy).toHaveBeenCalled();
      expect(mockTauri.startProxy).toHaveBeenCalled();
      expect(result.running).toBe(true);
    });
  });

  describe("Server-only methods throw appropriate errors", () => {
    it("listUsers throws UNSUPPORTED_IN_TAURI", async () => {
      try {
        await tauriClient.listUsers();
        expect.fail("Should have thrown");
      } catch (e) {
        expect(isBackendError(e)).toBe(true);
        const error = e as BackendError;
        expect(error.code).toBe("UNSUPPORTED_IN_TAURI");
        expect(error.message).toContain("listUsers");
      }
    });

    it("createUser throws UNSUPPORTED_IN_TAURI", async () => {
      try {
        await tauriClient.createUser({ name: "test" });
        expect.fail("Should have thrown");
      } catch (e) {
        expect(isBackendError(e)).toBe(true);
        expect((e as BackendError).code).toBe("UNSUPPORTED_IN_TAURI");
      }
    });

    it("getUser throws UNSUPPORTED_IN_TAURI", async () => {
      try {
        await tauriClient.getUser(1);
        expect.fail("Should have thrown");
      } catch (e) {
        expect(isBackendError(e)).toBe(true);
        expect((e as BackendError).code).toBe("UNSUPPORTED_IN_TAURI");
      }
    });

    it("updateUser throws UNSUPPORTED_IN_TAURI", async () => {
      try {
        await tauriClient.updateUser(1, { name: "new" });
        expect.fail("Should have thrown");
      } catch (e) {
        expect(isBackendError(e)).toBe(true);
        expect((e as BackendError).code).toBe("UNSUPPORTED_IN_TAURI");
      }
    });

    it("deleteUser throws UNSUPPORTED_IN_TAURI", async () => {
      try {
        await tauriClient.deleteUser(1);
        expect.fail("Should have thrown");
      } catch (e) {
        expect(isBackendError(e)).toBe(true);
        expect((e as BackendError).code).toBe("UNSUPPORTED_IN_TAURI");
      }
    });

    it("regenerateApiKey throws UNSUPPORTED_IN_TAURI", async () => {
      try {
        await tauriClient.regenerateApiKey(1);
        expect.fail("Should have thrown");
      } catch (e) {
        expect(isBackendError(e)).toBe(true);
        expect((e as BackendError).code).toBe("UNSUPPORTED_IN_TAURI");
      }
    });

    it("resetUserUsage throws UNSUPPORTED_IN_TAURI", async () => {
      try {
        await tauriClient.resetUserUsage(1);
        expect.fail("Should have thrown");
      } catch (e) {
        expect(isBackendError(e)).toBe(true);
        expect((e as BackendError).code).toBe("UNSUPPORTED_IN_TAURI");
      }
    });

    it("getUserUsage throws UNSUPPORTED_IN_TAURI", async () => {
      try {
        await tauriClient.getUserUsage(1);
        expect.fail("Should have thrown");
      } catch (e) {
        expect(isBackendError(e)).toBe(true);
        expect((e as BackendError).code).toBe("UNSUPPORTED_IN_TAURI");
      }
    });

    it("getRequestLogs throws UNSUPPORTED_IN_TAURI", async () => {
      try {
        await tauriClient.getRequestLogs?.();
        expect.fail("Should have thrown");
      } catch (e) {
        expect(isBackendError(e)).toBe(true);
        expect((e as BackendError).code).toBe("UNSUPPORTED_IN_TAURI");
      }
    });
  });

  describe("Provider Management", () => {
    it("listProviders maps auth status to provider info", async () => {
      mockTauri.getAuthStatus.mockResolvedValue({
        claude: 2,
        openai: 1,
        gemini: 0,
        qwen: 0,
        iflow: 0,
        vertex: 1,
        antigravity: 0,
      });

      const result = await tauriClient.listProviders();

      expect(result.providers).toHaveLength(7);
      const claude = result.providers.find((p) => p.name === "claude");
      expect(claude?.status).toBe("active");
      expect(claude?.accounts).toBe(2);

      const gemini = result.providers.find((p) => p.name === "gemini");
      expect(gemini?.status).toBe("inactive");
      expect(gemini?.accounts).toBe(0);
    });

    it("startOAuth calls openOAuth", async () => {
      mockTauri.openOAuth.mockResolvedValue("state-123");

      await tauriClient.startOAuth("claude");

      expect(mockTauri.openOAuth).toHaveBeenCalledWith("claude");
    });

    it("removeProviderAccount calls disconnectProvider", async () => {
      mockTauri.disconnectProvider.mockResolvedValue({
        claude: 0,
        openai: 0,
        gemini: 0,
        qwen: 0,
        iflow: 0,
        vertex: 0,
        antigravity: 0,
      });

      await tauriClient.removeProviderAccount("claude", 1);

      expect(mockTauri.disconnectProvider).toHaveBeenCalledWith("claude");
    });
  });

  describe("Configuration", () => {
    it("getConfig maps Tauri config to normalized config", async () => {
      mockTauri.getConfig.mockResolvedValue({
        port: 8080,
        autoStart: true,
        launchAtLogin: false,
        debug: false,
        proxyUrl: "",
        requestRetry: 3,
        quotaSwitchProject: false,
        quotaSwitchPreviewModel: false,
        usageStatsEnabled: true,
        requestLogging: true,
        loggingToFile: false,
        ampApiKey: "",
        ampModelMappings: [{ from: "gpt-4", to: "claude-3-opus", enabled: true }],
        ampOpenaiProviders: [],
        ampRoutingMode: "mappings",
        copilot: { enabled: false, port: 5000, accountType: "", githubToken: "", rateLimitWait: false },
      });

      const result = await tauriClient.getConfig();

      expect(result.proxyPort).toBe(8080);
      expect(result.autoStartProxy).toBe(true);
      expect(result.modelMappings).toEqual({ "gpt-4": "claude-3-opus" });
    });

    it("saveConfig updates and saves config", async () => {
      const currentConfig = {
        port: 8080,
        autoStart: false,
        launchAtLogin: false,
        debug: false,
        proxyUrl: "",
        requestRetry: 3,
        quotaSwitchProject: false,
        quotaSwitchPreviewModel: false,
        usageStatsEnabled: true,
        requestLogging: true,
        loggingToFile: false,
        ampApiKey: "",
        ampModelMappings: [],
        ampOpenaiProviders: [],
        ampRoutingMode: "mappings",
        copilot: { enabled: false, port: 5000, accountType: "", githubToken: "", rateLimitWait: false },
      };
      mockTauri.getConfig.mockResolvedValue(currentConfig);
      mockTauri.saveConfig.mockResolvedValue(undefined);

      const result = await tauriClient.saveConfig({ proxyPort: 9090 });

      expect(mockTauri.saveConfig).toHaveBeenCalledWith(
        expect.objectContaining({ port: 9090 })
      );
      expect(result.success).toBe(true);
      expect(result.restartRequired).toBe(true);
    });
  });

  describe("Usage Statistics", () => {
    it("getUsageStats maps Tauri stats to normalized format", async () => {
      mockTauri.getUsageStats.mockResolvedValue({
        totalRequests: 100,
        successCount: 95,
        failureCount: 5,
        totalTokens: 50000,
        inputTokens: 30000,
        outputTokens: 20000,
        requestsToday: 10,
        tokensToday: 5000,
        models: [
          { model: "claude-3-opus", requests: 50, tokens: 25000 },
          { model: "gpt-4", requests: 50, tokens: 25000 },
        ],
        requestsByDay: [{ label: "2024-01-01", value: 10 }],
        tokensByDay: [{ label: "2024-01-01", value: 5000 }],
        requestsByHour: [],
        tokensByHour: [],
      });

      const result = await tauriClient.getUsageStats();

      expect(result.totalRequests).toBe(100);
      expect(result.totalTokensInput).toBe(30000);
      expect(result.totalTokensOutput).toBe(20000);
      expect(result.byProvider?.claude.requests).toBe(50);
      expect(result.byProvider?.openai.requests).toBe(50);
    });

    it("getDailyUsage returns formatted daily data", async () => {
      mockTauri.getUsageStats.mockResolvedValue({
        totalRequests: 100,
        successCount: 95,
        failureCount: 5,
        totalTokens: 50000,
        inputTokens: 30000,
        outputTokens: 20000,
        requestsToday: 10,
        tokensToday: 5000,
        models: [],
        requestsByDay: [
          { label: "2024-01-01", value: 10 },
          { label: "2024-01-02", value: 20 },
        ],
        tokensByDay: [
          { label: "2024-01-01", value: 1000 },
          { label: "2024-01-02", value: 2000 },
        ],
        requestsByHour: [],
        tokensByHour: [],
      });

      const result = await tauriClient.getDailyUsage({ days: 7 });

      expect(result.days).toBe(7);
      expect(result.data).toHaveLength(2);
      expect(result.data[0].date).toBe("2024-01-01");
      expect(result.data[0].requests).toBe(10);
      expect(result.data[0].tokensInput).toBe(1000);
    });
  });

  describe("Authentication", () => {
    it("login always returns success in Tauri mode", async () => {
      const result = await tauriClient.login("password");
      expect(result.success).toBe(true);
    });

    it("logout is a no-op", async () => {
      await expect(tauriClient.logout()).resolves.toBeUndefined();
    });

    it("getAuthStatus returns authenticated true", async () => {
      const result = await tauriClient.getAuthStatus();
      expect(result.authenticated).toBe(true);
    });
  });

  describe("Event Listeners", () => {
    it("onProxyStatusChanged returns unsubscribe function", async () => {
      const unlisten = vi.fn();
      mockTauri.onProxyStatusChanged.mockResolvedValue(unlisten);

      const result = await tauriClient.onProxyStatusChanged(() => {});

      expect(mockTauri.onProxyStatusChanged).toHaveBeenCalled();
      expect(typeof result).toBe("function");
    });

    it("onProxyStatusChanged maps status in callback", async () => {
      let capturedCallback: (status: tauri.ProxyStatus) => void;
      mockTauri.onProxyStatusChanged.mockImplementation((cb: (status: tauri.ProxyStatus) => void) => {
        capturedCallback = cb;
        return Promise.resolve(() => {});
      });

      const receivedStatuses: Array<{ running: boolean; port: number }> = [];
      await tauriClient.onProxyStatusChanged((status) => {
        receivedStatuses.push(status);
      });

      capturedCallback!({
        running: true,
        port: 8080,
        endpoint: "http://localhost:8080",
      });

      expect(receivedStatuses).toHaveLength(1);
      expect(receivedStatuses[0].running).toBe(true);
      expect(receivedStatuses[0].port).toBe(8080);
    });

    it("onAuthStatusChanged returns unsubscribe function", async () => {
      const unlisten = vi.fn();
      mockTauri.onAuthStatusChanged.mockResolvedValue(unlisten);

      const result = await tauriClient.onAuthStatusChanged?.(() => {});

      expect(mockTauri.onAuthStatusChanged).toHaveBeenCalled();
      expect(typeof result).toBe("function");
    });

    it("onRequestLog returns unsubscribe function", async () => {
      const unlisten = vi.fn();
      mockTauri.onRequestLog.mockResolvedValue(unlisten);

      const result = await tauriClient.onRequestLog?.(() => {});

      expect(mockTauri.onRequestLog).toHaveBeenCalled();
      expect(typeof result).toBe("function");
    });
  });
});
