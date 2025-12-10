// Tauri client implementation - wraps existing tauri.ts invoke calls

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";

import type {
  BackendClient,
  ProxyStatus,
  Provider,
  AuthStatus,
  AppConfig,
  OAuthCallback,
  CopilotStatus,
  CopilotApiDetection,
  CopilotApiInstallResult,
  RequestLog,
  ProviderHealth,
  DetectedTool,
  ToolSetupInfo,
  AgentStatus,
  AgentConfigResult,
  UsageStats,
  RequestHistory,
  AgentTestResult,
  ProviderTestResult,
  GeminiApiKey,
  ClaudeApiKey,
  CodexApiKey,
  OpenAICompatibleProvider,
  AuthFile,
  LogEntry,
  AvailableModel,
  OAuthExcludedModels,
} from "./types";

export const tauriClient: BackendClient = {
  // Proxy management
  startProxy: () => invoke<ProxyStatus>("start_proxy"),
  stopProxy: () => invoke<ProxyStatus>("stop_proxy"),
  getProxyStatus: () => invoke<ProxyStatus>("get_proxy_status"),

  // OAuth management
  openOAuth: (provider: Provider) => invoke<string>("open_oauth", { provider }),
  pollOAuthStatus: (oauthState: string) =>
    invoke<boolean>("poll_oauth_status", { oauthState }),
  completeOAuth: (provider: Provider, code: string) =>
    invoke<AuthStatus>("complete_oauth", { provider, code }),
  disconnectProvider: (provider: Provider) =>
    invoke<AuthStatus>("disconnect_provider", { provider }),
  importVertexCredential: (filePath: string) =>
    invoke<AuthStatus>("import_vertex_credential", { filePath }),
  getAuthStatus: () => invoke<AuthStatus>("get_auth_status"),
  refreshAuthStatus: () => invoke<AuthStatus>("refresh_auth_status"),

  // Config
  getConfig: () => invoke<AppConfig>("get_config"),
  saveConfig: (config: AppConfig) => invoke<void>("save_config", { config }),

  // Event listeners
  async onProxyStatusChanged(
    callback: (status: ProxyStatus) => void,
  ): Promise<UnlistenFn> {
    return listen<ProxyStatus>("proxy-status-changed", (event) => {
      callback(event.payload);
    });
  },

  async onAuthStatusChanged(
    callback: (status: AuthStatus) => void,
  ): Promise<UnlistenFn> {
    return listen<AuthStatus>("auth-status-changed", (event) => {
      callback(event.payload);
    });
  },

  async onOAuthCallback(
    callback: (data: OAuthCallback) => void,
  ): Promise<UnlistenFn> {
    return listen<OAuthCallback>("oauth-callback", (event) => {
      callback(event.payload);
    });
  },

  async onTrayToggleProxy(
    callback: (shouldStart: boolean) => void,
  ): Promise<UnlistenFn> {
    return listen<boolean>("tray-toggle-proxy", (event) => {
      callback(event.payload);
    });
  },

  async onCopilotStatusChanged(
    callback: (status: CopilotStatus) => void,
  ): Promise<UnlistenFn> {
    return listen<CopilotStatus>("copilot-status-changed", (event) => {
      callback(event.payload);
    });
  },

  async onCopilotAuthRequired(
    callback: (message: string) => void,
  ): Promise<UnlistenFn> {
    return listen<string>("copilot-auth-required", (event) => {
      callback(event.payload);
    });
  },

  async onRequestLog(callback: (log: RequestLog) => void): Promise<UnlistenFn> {
    return listen<RequestLog>("request-log", (event) => {
      callback(event.payload);
    });
  },

  // Copilot management
  getCopilotStatus: () => invoke<CopilotStatus>("get_copilot_status"),
  startCopilot: () => invoke<CopilotStatus>("start_copilot"),
  stopCopilot: () => invoke<CopilotStatus>("stop_copilot"),
  checkCopilotHealth: () => invoke<CopilotStatus>("check_copilot_health"),
  detectCopilotApi: () => invoke<CopilotApiDetection>("detect_copilot_api"),
  installCopilotApi: () => invoke<CopilotApiInstallResult>("install_copilot_api"),

  // System notifications
  async showSystemNotification(title: string, body?: string): Promise<void> {
    let permissionGranted = await isPermissionGranted();

    if (!permissionGranted) {
      const permission = await requestPermission();
      permissionGranted = permission === "granted";
    }

    if (permissionGranted) {
      sendNotification({ title, body });
    }
  },

  // Provider health
  checkProviderHealth: () => invoke<ProviderHealth>("check_provider_health"),

  // AI Tool detection
  detectAiTools: () => invoke<DetectedTool[]>("detect_ai_tools"),
  configureContinue: () => invoke<string>("configure_continue"),
  getToolSetupInfo: (toolId: string) =>
    invoke<ToolSetupInfo>("get_tool_setup_info", { toolId }),

  // CLI Agents
  detectCliAgents: () => invoke<AgentStatus[]>("detect_cli_agents"),
  configureCliAgent: (agentId: string, models: AvailableModel[]) =>
    invoke<AgentConfigResult>("configure_cli_agent", { agentId, models }),
  getShellProfilePath: () => invoke<string>("get_shell_profile_path"),
  appendToShellProfile: (content: string) =>
    invoke<string>("append_to_shell_profile", { content }),

  // Usage Statistics
  getUsageStats: () => invoke<UsageStats>("get_usage_stats"),
  getRequestHistory: () => invoke<RequestHistory>("get_request_history"),
  addRequestToHistory: (request: RequestLog) =>
    invoke<RequestHistory>("add_request_to_history", { request }),
  clearRequestHistory: () => invoke<void>("clear_request_history"),
  syncUsageFromProxy: () => invoke<RequestHistory>("sync_usage_from_proxy"),

  // Agent testing
  testAgentConnection: (agentId: string) =>
    invoke<AgentTestResult>("test_agent_connection", { agentId }),
  testOpenAIProvider: (baseUrl: string, apiKey: string) =>
    invoke<ProviderTestResult>("test_openai_provider", { baseUrl, apiKey }),

  // API Keys Management
  getGeminiApiKeys: () => invoke<GeminiApiKey[]>("get_gemini_api_keys"),
  setGeminiApiKeys: (keys: GeminiApiKey[]) =>
    invoke<void>("set_gemini_api_keys", { keys }),
  addGeminiApiKey: (key: GeminiApiKey) =>
    invoke<void>("add_gemini_api_key", { key }),
  deleteGeminiApiKey: (index: number) =>
    invoke<void>("delete_gemini_api_key", { index }),

  getClaudeApiKeys: () => invoke<ClaudeApiKey[]>("get_claude_api_keys"),
  setClaudeApiKeys: (keys: ClaudeApiKey[]) =>
    invoke<void>("set_claude_api_keys", { keys }),
  addClaudeApiKey: (key: ClaudeApiKey) =>
    invoke<void>("add_claude_api_key", { key }),
  deleteClaudeApiKey: (index: number) =>
    invoke<void>("delete_claude_api_key", { index }),

  getCodexApiKeys: () => invoke<CodexApiKey[]>("get_codex_api_keys"),
  setCodexApiKeys: (keys: CodexApiKey[]) =>
    invoke<void>("set_codex_api_keys", { keys }),
  addCodexApiKey: (key: CodexApiKey) =>
    invoke<void>("add_codex_api_key", { key }),
  deleteCodexApiKey: (index: number) =>
    invoke<void>("delete_codex_api_key", { index }),

  getOpenAICompatibleProviders: () =>
    invoke<OpenAICompatibleProvider[]>("get_openai_compatible_providers"),
  setOpenAICompatibleProviders: (providers: OpenAICompatibleProvider[]) =>
    invoke<void>("set_openai_compatible_providers", { providers }),
  addOpenAICompatibleProvider: (provider: OpenAICompatibleProvider) =>
    invoke<void>("add_openai_compatible_provider", { provider }),
  deleteOpenAICompatibleProvider: (index: number) =>
    invoke<void>("delete_openai_compatible_provider", { index }),

  // Auth Files Management
  getAuthFiles: () => invoke<AuthFile[]>("get_auth_files"),
  uploadAuthFile: (filePath: string, provider: string) =>
    invoke<void>("upload_auth_file", { filePath, provider }),
  deleteAuthFile: (fileId: string) =>
    invoke<void>("delete_auth_file", { fileId }),
  toggleAuthFile: (fileId: string, disabled: boolean) =>
    invoke<void>("toggle_auth_file", { fileId, disabled }),
  downloadAuthFile: (fileId: string, filename: string) =>
    invoke<string>("download_auth_file", { fileId, filename }),
  deleteAllAuthFiles: () => invoke<void>("delete_all_auth_files"),

  // Logs
  getLogs: (lines?: number) => invoke<LogEntry[]>("get_logs", { lines }),
  clearLogs: () => invoke<void>("clear_logs"),

  // Available Models
  getAvailableModels: () => invoke<AvailableModel[]>("get_available_models"),

  // Management API Settings
  getMaxRetryInterval: () => invoke<number>("get_max_retry_interval"),
  setMaxRetryInterval: (value: number) =>
    invoke<void>("set_max_retry_interval", { value }),
  getWebsocketAuth: () => invoke<boolean>("get_websocket_auth"),
  setWebsocketAuth: (value: boolean) =>
    invoke<void>("set_websocket_auth", { value }),
  getPrioritizeModelMappings: () =>
    invoke<boolean>("get_prioritize_model_mappings"),
  setPrioritizeModelMappings: (value: boolean) =>
    invoke<void>("set_prioritize_model_mappings", { value }),
  getOAuthExcludedModels: () =>
    invoke<OAuthExcludedModels>("get_oauth_excluded_models"),
  setOAuthExcludedModels: (provider: string, models: string[]) =>
    invoke<void>("set_oauth_excluded_models", { provider, models }),
  deleteOAuthExcludedModels: (provider: string) =>
    invoke<void>("delete_oauth_excluded_models", { provider }),
  getConfigYaml: () => invoke<string>("get_config_yaml"),
  setConfigYaml: (yaml: string) => invoke<void>("set_config_yaml", { yaml }),
  getRequestErrorLogs: () => invoke<string[]>("get_request_error_logs"),
  getRequestErrorLogContent: (filename: string) =>
    invoke<string>("get_request_error_log_content", { filename }),
};
