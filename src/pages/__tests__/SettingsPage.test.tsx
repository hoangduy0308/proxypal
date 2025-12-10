/**
 * Tests for SettingsPage component
 */

import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { render, screen, fireEvent, waitFor, cleanup } from "@solidjs/testing-library";

vi.mock("../../backend", () => ({
  isTauri: vi.fn(),
  backendClient: {
    getConfig: vi.fn(),
    saveConfig: vi.fn(),
  },
}));

vi.mock("../../stores/app", () => ({
  appStore: {
    proxyStatus: vi.fn(),
    authStatus: vi.fn(),
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
  saveConfig: vi.fn(),
  AMP_MODEL_SLOTS: [],
  testOpenAIProvider: vi.fn(),
  getAvailableModels: vi.fn().mockResolvedValue([]),
  getMaxRetryInterval: vi.fn().mockResolvedValue(0),
  setMaxRetryInterval: vi.fn(),
  getWebsocketAuth: vi.fn().mockResolvedValue(false),
  setWebsocketAuth: vi.fn(),
  getPrioritizeModelMappings: vi.fn().mockResolvedValue(false),
  setPrioritizeModelMappings: vi.fn(),
  getOAuthExcludedModels: vi.fn().mockResolvedValue({}),
  setOAuthExcludedModels: vi.fn(),
  deleteOAuthExcludedModels: vi.fn(),
  getConfigYaml: vi.fn().mockResolvedValue(""),
  setConfigYaml: vi.fn(),
  detectCopilotApi: vi.fn(),
}));

import { SettingsPage } from "../Settings";
import { isTauri, backendClient } from "../../backend";
import { appStore } from "../../stores/app";
import { saveConfig } from "../../lib/tauri";

const mockIsTauri = isTauri as unknown as ReturnType<typeof vi.fn>;
const mockBackendClient = backendClient as unknown as {
  getConfig: ReturnType<typeof vi.fn>;
  saveConfig: ReturnType<typeof vi.fn>;
};
const mockAppStore = appStore as unknown as {
  proxyStatus: ReturnType<typeof vi.fn>;
  authStatus: ReturnType<typeof vi.fn>;
  config: ReturnType<typeof vi.fn>;
  setConfig: ReturnType<typeof vi.fn>;
  setCurrentPage: ReturnType<typeof vi.fn>;
};
const mockSaveConfig = saveConfig as unknown as ReturnType<typeof vi.fn>;

describe("SettingsPage", () => {
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
      launchAtLogin: false,
      debug: false,
      proxyUrl: "",
      requestRetry: 0,
      ampApiKey: "",
      ampModelMappings: [],
      ampOpenaiProviders: [],
      copilot: { enabled: false, port: 4141, accountType: "individual" },
    });
  });

  afterEach(() => {
    cleanup();
  });

  describe("Settings rendering", () => {
    it("renders settings page with title", async () => {
      mockIsTauri.mockReturnValue(true);

      render(() => <SettingsPage />);

      expect(screen.getByText("Settings")).toBeInTheDocument();
    });

    it("renders General section", async () => {
      mockIsTauri.mockReturnValue(true);

      render(() => <SettingsPage />);

      expect(screen.getByText("General")).toBeInTheDocument();
    });

    it("renders Proxy Configuration section", async () => {
      mockIsTauri.mockReturnValue(true);

      render(() => <SettingsPage />);

      expect(screen.getByText("Proxy Configuration")).toBeInTheDocument();
    });
  });

  describe("Config save via adapter", () => {
    it("saves config via Tauri in Tauri mode", async () => {
      mockIsTauri.mockReturnValue(true);
      mockSaveConfig.mockResolvedValue(undefined);

      render(() => <SettingsPage />);

      const portInput = screen.getByDisplayValue("8317");
      fireEvent.input(portInput, { target: { value: "8318" } });

      await waitFor(() => {
        expect(mockSaveConfig).toHaveBeenCalled();
      });
    });

    it("saves config via HTTP client in HTTP mode", async () => {
      mockIsTauri.mockReturnValue(false);
      mockBackendClient.saveConfig.mockResolvedValue({ success: true, restartRequired: false });

      render(() => <SettingsPage />);

      const portInput = screen.getByDisplayValue("8317");
      fireEvent.input(portInput, { target: { value: "8318" } });

      await waitFor(() => {
        expect(mockBackendClient.saveConfig).toHaveBeenCalled();
      });
    });
  });

  describe("Tauri-only features", () => {
    it("shows Launch at login option in Tauri mode", async () => {
      mockIsTauri.mockReturnValue(true);

      render(() => <SettingsPage />);

      expect(screen.getByText("Launch at login")).toBeInTheDocument();
    });

    it("hides Launch at login option in HTTP mode", async () => {
      mockIsTauri.mockReturnValue(false);

      render(() => <SettingsPage />);

      expect(screen.queryByText("Launch at login")).not.toBeInTheDocument();
    });

    it("shows desktop-only info message in HTTP mode", async () => {
      mockIsTauri.mockReturnValue(false);

      render(() => <SettingsPage />);

      expect(screen.getByText("Some settings are only available in the desktop app")).toBeInTheDocument();
    });

    it("shows Auth Files section in Tauri mode", async () => {
      mockIsTauri.mockReturnValue(true);

      render(() => <SettingsPage />);

      expect(screen.getByText("Auth Files")).toBeInTheDocument();
    });

    it("hides Auth Files section in HTTP mode", async () => {
      mockIsTauri.mockReturnValue(false);

      render(() => <SettingsPage />);

      expect(screen.queryByText("Auth Files")).not.toBeInTheDocument();
    });

    it("shows Copilot API Detection in Tauri mode", async () => {
      mockIsTauri.mockReturnValue(true);

      render(() => <SettingsPage />);

      expect(screen.getByText("Copilot API Detection")).toBeInTheDocument();
    });

    it("hides Copilot API Detection in HTTP mode", async () => {
      mockIsTauri.mockReturnValue(false);

      render(() => <SettingsPage />);

      expect(screen.queryByText("Copilot API Detection")).not.toBeInTheDocument();
    });
  });

  describe("Runtime settings visibility", () => {
    it("shows Max Retry Interval when proxy running in Tauri mode", async () => {
      mockIsTauri.mockReturnValue(true);
      mockAppStore.proxyStatus.mockReturnValue({
        running: true,
        port: 8317,
        endpoint: "http://localhost:8317/v1",
      });

      render(() => <SettingsPage />);

      await waitFor(() => {
        expect(screen.getByText("Max Retry Interval (seconds)")).toBeInTheDocument();
      });
    });

    it("hides Max Retry Interval when proxy running in HTTP mode", async () => {
      mockIsTauri.mockReturnValue(false);
      mockAppStore.proxyStatus.mockReturnValue({
        running: true,
        port: 8317,
        endpoint: "http://localhost:8317/v1",
      });

      render(() => <SettingsPage />);

      expect(screen.queryByText("Max Retry Interval (seconds)")).not.toBeInTheDocument();
    });

    it("hides Prioritize Model Mappings toggle in HTTP mode", async () => {
      mockIsTauri.mockReturnValue(false);
      mockAppStore.proxyStatus.mockReturnValue({
        running: true,
        port: 8317,
        endpoint: "http://localhost:8317/v1",
      });

      render(() => <SettingsPage />);

      expect(screen.queryByText("Prioritize Model Mappings")).not.toBeInTheDocument();
    });

    it("hides Raw YAML Config Editor in HTTP mode", async () => {
      mockIsTauri.mockReturnValue(false);
      mockAppStore.proxyStatus.mockReturnValue({
        running: true,
        port: 8317,
        endpoint: "http://localhost:8317/v1",
      });

      render(() => <SettingsPage />);

      expect(screen.queryByText("Raw Configuration")).not.toBeInTheDocument();
    });
  });

  describe("Common settings", () => {
    it("shows Auto-start proxy option in both modes", async () => {
      mockIsTauri.mockReturnValue(false);

      render(() => <SettingsPage />);

      expect(screen.getByText("Auto-start proxy")).toBeInTheDocument();
    });

    it("shows Port configuration in both modes", async () => {
      mockIsTauri.mockReturnValue(false);

      render(() => <SettingsPage />);

      expect(screen.getByText("Port")).toBeInTheDocument();
    });

    it("shows Connected Accounts section in both modes", async () => {
      mockIsTauri.mockReturnValue(false);

      render(() => <SettingsPage />);

      expect(screen.getByText("Connected Accounts")).toBeInTheDocument();
    });

    it("shows API Keys section in both modes", async () => {
      mockIsTauri.mockReturnValue(false);

      render(() => <SettingsPage />);

      expect(screen.getByText("API Keys")).toBeInTheDocument();
    });
  });
});
