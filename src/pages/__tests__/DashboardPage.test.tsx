/**
 * Tests for DashboardPage component
 */

import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { render, screen, fireEvent, waitFor, cleanup } from "@solidjs/testing-library";

vi.mock("../../backend", () => ({
  isTauri: vi.fn(),
  backendClient: {
    getProxyStatus: vi.fn(),
    startProxy: vi.fn(),
    stopProxy: vi.fn(),
    getUsageStats: vi.fn(),
    getRequestLogs: vi.fn(),
  },
}));

vi.mock("../../stores/app", () => ({
  appStore: {
    proxyStatus: vi.fn(),
    setProxyStatus: vi.fn(),
    authStatus: vi.fn(),
    setAuthStatus: vi.fn(),
    config: vi.fn(),
    setConfig: vi.fn(),
    setCurrentPage: vi.fn(),
  },
}));

vi.mock("../../stores/toast", () => ({
  toastStore: {
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
    warning: vi.fn(),
  },
}));

vi.mock("../../lib/tauri", () => ({
  startProxy: vi.fn(),
  stopProxy: vi.fn(),
  getRequestHistory: vi.fn(),
  getUsageStats: vi.fn(),
  syncUsageFromProxy: vi.fn(),
  onRequestLog: vi.fn(),
  detectCliAgents: vi.fn(),
  getAvailableModels: vi.fn(),
  openOAuth: vi.fn(),
  pollOAuthStatus: vi.fn(),
  disconnectProvider: vi.fn(),
  refreshAuthStatus: vi.fn(),
  importVertexCredential: vi.fn(),
  configureCliAgent: vi.fn(),
  testAgentConnection: vi.fn(),
  appendToShellProfile: vi.fn(),
  onCopilotStatusChanged: vi.fn(),
  getCopilotStatus: vi.fn(),
  startCopilotProxy: vi.fn(),
  stopCopilotProxy: vi.fn(),
  onCopilotAuthRequired: vi.fn(),
  detectCopilotApi: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(),
}));

import { DashboardPage } from "../Dashboard";
import { isTauri, backendClient } from "../../backend";
import { appStore } from "../../stores/app";
import {
  startProxy,
  stopProxy,
  getRequestHistory,
  getUsageStats,
  detectCliAgents,
  onRequestLog,
  onCopilotStatusChanged,
  getCopilotStatus,
  onCopilotAuthRequired,
  detectCopilotApi,
} from "../../lib/tauri";

const mockIsTauri = isTauri as unknown as ReturnType<typeof vi.fn>;
const mockBackendClient = backendClient as unknown as {
  startProxy: ReturnType<typeof vi.fn>;
  stopProxy: ReturnType<typeof vi.fn>;
  getUsageStats: ReturnType<typeof vi.fn>;
  getRequestLogs: ReturnType<typeof vi.fn>;
};
const mockAppStore = appStore as unknown as {
  proxyStatus: ReturnType<typeof vi.fn>;
  setProxyStatus: ReturnType<typeof vi.fn>;
  authStatus: ReturnType<typeof vi.fn>;
  setAuthStatus: ReturnType<typeof vi.fn>;
  config: ReturnType<typeof vi.fn>;
  setConfig: ReturnType<typeof vi.fn>;
  setCurrentPage: ReturnType<typeof vi.fn>;
};
const mockStartProxy = startProxy as unknown as ReturnType<typeof vi.fn>;
const mockStopProxy = stopProxy as unknown as ReturnType<typeof vi.fn>;
const mockGetRequestHistory = getRequestHistory as unknown as ReturnType<typeof vi.fn>;
const mockGetUsageStats = getUsageStats as unknown as ReturnType<typeof vi.fn>;
const mockDetectCliAgents = detectCliAgents as unknown as ReturnType<typeof vi.fn>;
const mockOnRequestLog = onRequestLog as unknown as ReturnType<typeof vi.fn>;
const mockOnCopilotStatusChanged = onCopilotStatusChanged as unknown as ReturnType<typeof vi.fn>;
const mockGetCopilotStatus = getCopilotStatus as unknown as ReturnType<typeof vi.fn>;
const mockOnCopilotAuthRequired = onCopilotAuthRequired as unknown as ReturnType<typeof vi.fn>;
const mockDetectCopilotApi = detectCopilotApi as unknown as ReturnType<typeof vi.fn>;

describe("DashboardPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    
    mockAppStore.proxyStatus.mockReturnValue({
      running: false,
      port: 8317,
      endpoint: "http://localhost:8317/v1",
    });
    mockAppStore.authStatus.mockReturnValue({
      claude: 0,
      openai: 0,
      gemini: 0,
      qwen: 0,
      iflow: 0,
      vertex: 0,
      antigravity: 0,
    });
    mockAppStore.config.mockReturnValue({
      port: 8317,
      autoStart: true,
      copilot: { enabled: false, port: 4141, accountType: "individual" },
    });
    mockOnRequestLog.mockResolvedValue(() => {});
    mockGetRequestHistory.mockResolvedValue({
      requests: [],
      totalTokensIn: 0,
      totalTokensOut: 0,
      totalCostUsd: 0,
    });
    mockGetUsageStats.mockResolvedValue({
      totalRequests: 0,
      totalTokensIn: 0,
      totalTokensOut: 0,
      totalCost: 0,
      byProvider: {},
      byModel: {},
    });
    mockDetectCliAgents.mockResolvedValue([]);
    mockOnCopilotStatusChanged.mockResolvedValue(() => {});
    mockGetCopilotStatus.mockResolvedValue({ running: false, port: 4141 });
    mockOnCopilotAuthRequired.mockResolvedValue(() => {});
    mockDetectCopilotApi.mockResolvedValue({ installed: false, path: null });
  });

  afterEach(() => {
    cleanup();
  });

  describe("Proxy status display", () => {
    it("shows proxy stopped status", async () => {
      mockIsTauri.mockReturnValue(true);
      mockAppStore.proxyStatus.mockReturnValue({
        running: false,
        port: 8317,
        endpoint: "http://localhost:8317/v1",
      });

      render(() => <DashboardPage />);

      await waitFor(() => {
        expect(screen.getByText("Stopped")).toBeInTheDocument();
      });
    });

    it("shows proxy running status", async () => {
      mockIsTauri.mockReturnValue(true);
      mockAppStore.proxyStatus.mockReturnValue({
        running: true,
        port: 8317,
        endpoint: "http://localhost:8317/v1",
      });

      render(() => <DashboardPage />);

      await waitFor(() => {
        expect(screen.getByText("Running")).toBeInTheDocument();
      });
    });
  });

  describe("Start/Stop proxy", () => {
    it("starts proxy in Tauri mode", async () => {
      mockIsTauri.mockReturnValue(true);
      mockAppStore.proxyStatus.mockReturnValue({
        running: false,
        port: 8317,
        endpoint: "http://localhost:8317/v1",
      });
      mockStartProxy.mockResolvedValue({
        running: true,
        port: 8317,
        endpoint: "http://localhost:8317/v1",
      });

      render(() => <DashboardPage />);

      const startButton = screen.getByText("Start");
      fireEvent.click(startButton);

      await waitFor(() => {
        expect(mockStartProxy).toHaveBeenCalled();
      });
    });

    it("starts proxy in HTTP mode by clicking status indicator", async () => {
      mockIsTauri.mockReturnValue(false);
      mockAppStore.proxyStatus.mockReturnValue({
        running: false,
        port: 8317,
        endpoint: "http://localhost:8317/v1",
      });
      mockBackendClient.getRequestLogs.mockResolvedValue({
        logs: [],
        total: 0,
        limit: 50,
        offset: 0,
      });
      mockBackendClient.getUsageStats.mockResolvedValue({
        period: "day",
        totalRequests: 0,
        totalTokensInput: 0,
        totalTokensOutput: 0,
      });

      render(() => <DashboardPage />);

      await waitFor(() => {
        expect(screen.getByText("Stopped")).toBeInTheDocument();
      });
    });

    it("stops proxy by clicking status indicator", async () => {
      mockIsTauri.mockReturnValue(true);
      mockAppStore.proxyStatus.mockReturnValue({
        running: true,
        port: 8317,
        endpoint: "http://localhost:8317/v1",
      });
      mockStopProxy.mockResolvedValue({
        running: false,
        port: 8317,
        endpoint: "http://localhost:8317/v1",
      });

      render(() => <DashboardPage />);

      const statusButton = screen.getByText("Running");
      fireEvent.click(statusButton);

      await waitFor(() => {
        expect(mockStopProxy).toHaveBeenCalled();
      });
    });
  });

  describe("Tauri-only features", () => {
    it("shows CLI agents section in Tauri mode", async () => {
      mockIsTauri.mockReturnValue(true);
      mockDetectCliAgents.mockResolvedValue([
        {
          id: "amp",
          name: "Amp",
          description: "Amp CLI agent",
          logo: "/logos/amp.svg",
          docsUrl: "https://docs.amp.dev",
          installed: true,
          configured: false,
        },
      ]);

      render(() => <DashboardPage />);

      await waitFor(() => {
        expect(screen.getByText("CLI Agents")).toBeInTheDocument();
      });
    });

    it("hides CLI agents section in HTTP mode", async () => {
      mockIsTauri.mockReturnValue(false);
      mockBackendClient.getRequestLogs.mockResolvedValue({
        logs: [],
        total: 0,
        limit: 50,
        offset: 0,
      });
      mockBackendClient.getUsageStats.mockResolvedValue({
        period: "day",
        totalRequests: 0,
        totalTokensInput: 0,
        totalTokensOutput: 0,
      });

      render(() => <DashboardPage />);

      await waitFor(() => {
        expect(screen.getByText("Agent configuration is only available in the desktop app")).toBeInTheDocument();
      });
    });

    it("shows desktop-only badge in HTTP mode for provider connection", async () => {
      mockIsTauri.mockReturnValue(false);
      mockBackendClient.getRequestLogs.mockResolvedValue({
        logs: [],
        total: 0,
        limit: 50,
        offset: 0,
      });
      mockBackendClient.getUsageStats.mockResolvedValue({
        period: "day",
        totalRequests: 0,
        totalTokensInput: 0,
        totalTokensOutput: 0,
      });

      render(() => <DashboardPage />);

      await waitFor(() => {
        expect(screen.getByText("Desktop only")).toBeInTheDocument();
      });
    });
  });

  describe("Usage statistics", () => {
    it("loads usage stats in Tauri mode", async () => {
      mockIsTauri.mockReturnValue(true);
      mockGetUsageStats.mockResolvedValue({
        totalRequests: 150,
        successCount: 150,
        failureCount: 0,
        totalTokens: 75000,
        inputTokens: 50000,
        outputTokens: 25000,
        requestsToday: 10,
        tokensToday: 5000,
        models: [],
        requestsByDay: [],
        tokensByDay: [],
        requestsByHour: [],
        tokensByHour: [],
      });

      render(() => <DashboardPage />);

      await waitFor(() => {
        expect(mockGetUsageStats).toHaveBeenCalled();
      });
    });

    it("loads usage stats in HTTP mode", async () => {
      mockIsTauri.mockReturnValue(false);
      mockBackendClient.getUsageStats.mockResolvedValue({
        period: "day",
        totalRequests: 200,
        totalTokensInput: 100000,
        totalTokensOutput: 50000,
      });
      mockBackendClient.getRequestLogs.mockResolvedValue({
        logs: [],
        total: 0,
        limit: 50,
        offset: 0,
      });

      render(() => <DashboardPage />);

      await waitFor(() => {
        expect(mockBackendClient.getUsageStats).toHaveBeenCalled();
      });
    });
  });
});
