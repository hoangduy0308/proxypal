// Backend adapter layer - auto-detects environment and exports appropriate client

import { httpClient } from "./httpClient";
import { tauriClient } from "./tauriClient";
import type { BackendClient } from "./types";

// Re-export all types
export * from "./types";

// Re-export constants from original tauri.ts for compatibility
export {
  AMP_MODEL_SLOTS,
  AMP_MODEL_ALIASES,
  COPILOT_MODELS,
} from "./constants";

// Detect if running in Tauri environment
export const isTauri =
  typeof window !== "undefined" && "__TAURI__" in window;

// Export the appropriate backend client based on environment
export const backend: BackendClient = isTauri ? tauriClient : httpClient;

// Also export individual clients for explicit use
export { httpClient } from "./httpClient";
export { tauriClient } from "./tauriClient";

// Export individual functions that match the original tauri.ts API
// This provides backward compatibility for existing code

// Proxy management
export const startProxy = () => backend.startProxy();
export const stopProxy = () => backend.stopProxy();
export const getProxyStatus = () => backend.getProxyStatus();

// OAuth management
export const openOAuth = (provider: Parameters<typeof backend.openOAuth>[0]) =>
  backend.openOAuth(provider);
export const pollOAuthStatus = (oauthState: string) =>
  backend.pollOAuthStatus(oauthState);
export const completeOAuth = (
  provider: Parameters<typeof backend.completeOAuth>[0],
  code: string,
) => backend.completeOAuth(provider, code);
export const disconnectProvider = (
  provider: Parameters<typeof backend.disconnectProvider>[0],
) => backend.disconnectProvider(provider);
export const importVertexCredential = (filePath: string) =>
  backend.importVertexCredential(filePath);
export const getAuthStatus = () => backend.getAuthStatus();
export const refreshAuthStatus = () => backend.refreshAuthStatus();

// Config
export const getConfig = () => backend.getConfig();
export const saveConfig = (config: Parameters<typeof backend.saveConfig>[0]) =>
  backend.saveConfig(config);

// Event listeners
export const onProxyStatusChanged = (
  callback: Parameters<typeof backend.onProxyStatusChanged>[0],
) => backend.onProxyStatusChanged(callback);
export const onAuthStatusChanged = (
  callback: Parameters<typeof backend.onAuthStatusChanged>[0],
) => backend.onAuthStatusChanged(callback);
export const onOAuthCallback = (
  callback: Parameters<typeof backend.onOAuthCallback>[0],
) => backend.onOAuthCallback(callback);
export const onTrayToggleProxy = (
  callback: Parameters<typeof backend.onTrayToggleProxy>[0],
) => backend.onTrayToggleProxy(callback);
export const onCopilotStatusChanged = (
  callback: Parameters<typeof backend.onCopilotStatusChanged>[0],
) => backend.onCopilotStatusChanged(callback);
export const onCopilotAuthRequired = (
  callback: Parameters<typeof backend.onCopilotAuthRequired>[0],
) => backend.onCopilotAuthRequired(callback);
export const onRequestLog = (
  callback: Parameters<typeof backend.onRequestLog>[0],
) => backend.onRequestLog(callback);

// Copilot management
export const getCopilotStatus = () => backend.getCopilotStatus();
export const startCopilot = () => backend.startCopilot();
export const stopCopilot = () => backend.stopCopilot();
export const checkCopilotHealth = () => backend.checkCopilotHealth();
export const detectCopilotApi = () => backend.detectCopilotApi();
export const installCopilotApi = () => backend.installCopilotApi();

// System notifications
export const showSystemNotification = (title: string, body?: string) =>
  backend.showSystemNotification(title, body);

// Provider health
export const checkProviderHealth = () => backend.checkProviderHealth();

// AI Tool detection
export const detectAiTools = () => backend.detectAiTools();
export const configureContinue = () => backend.configureContinue();
export const getToolSetupInfo = (toolId: string) =>
  backend.getToolSetupInfo(toolId);

// CLI Agents
export const detectCliAgents = () => backend.detectCliAgents();
export const configureCliAgent = (
  agentId: string,
  models: Parameters<typeof backend.configureCliAgent>[1],
) => backend.configureCliAgent(agentId, models);
export const getShellProfilePath = () => backend.getShellProfilePath();
export const appendToShellProfile = (content: string) =>
  backend.appendToShellProfile(content);

// Usage Statistics
export const getUsageStats = () => backend.getUsageStats();
export const getRequestHistory = () => backend.getRequestHistory();
export const addRequestToHistory = (
  request: Parameters<typeof backend.addRequestToHistory>[0],
) => backend.addRequestToHistory(request);
export const clearRequestHistory = () => backend.clearRequestHistory();
export const syncUsageFromProxy = () => backend.syncUsageFromProxy();

// Agent testing
export const testAgentConnection = (agentId: string) =>
  backend.testAgentConnection(agentId);
export const testOpenAIProvider = (baseUrl: string, apiKey: string) =>
  backend.testOpenAIProvider(baseUrl, apiKey);

// API Keys Management
export const getGeminiApiKeys = () => backend.getGeminiApiKeys();
export const setGeminiApiKeys = (
  keys: Parameters<typeof backend.setGeminiApiKeys>[0],
) => backend.setGeminiApiKeys(keys);
export const addGeminiApiKey = (
  key: Parameters<typeof backend.addGeminiApiKey>[0],
) => backend.addGeminiApiKey(key);
export const deleteGeminiApiKey = (index: number) =>
  backend.deleteGeminiApiKey(index);

export const getClaudeApiKeys = () => backend.getClaudeApiKeys();
export const setClaudeApiKeys = (
  keys: Parameters<typeof backend.setClaudeApiKeys>[0],
) => backend.setClaudeApiKeys(keys);
export const addClaudeApiKey = (
  key: Parameters<typeof backend.addClaudeApiKey>[0],
) => backend.addClaudeApiKey(key);
export const deleteClaudeApiKey = (index: number) =>
  backend.deleteClaudeApiKey(index);

export const getCodexApiKeys = () => backend.getCodexApiKeys();
export const setCodexApiKeys = (
  keys: Parameters<typeof backend.setCodexApiKeys>[0],
) => backend.setCodexApiKeys(keys);
export const addCodexApiKey = (
  key: Parameters<typeof backend.addCodexApiKey>[0],
) => backend.addCodexApiKey(key);
export const deleteCodexApiKey = (index: number) =>
  backend.deleteCodexApiKey(index);

export const getOpenAICompatibleProviders = () =>
  backend.getOpenAICompatibleProviders();
export const setOpenAICompatibleProviders = (
  providers: Parameters<typeof backend.setOpenAICompatibleProviders>[0],
) => backend.setOpenAICompatibleProviders(providers);
export const addOpenAICompatibleProvider = (
  provider: Parameters<typeof backend.addOpenAICompatibleProvider>[0],
) => backend.addOpenAICompatibleProvider(provider);
export const deleteOpenAICompatibleProvider = (index: number) =>
  backend.deleteOpenAICompatibleProvider(index);

// Auth Files Management
export const getAuthFiles = () => backend.getAuthFiles();
export const uploadAuthFile = (filePath: string, provider: string) =>
  backend.uploadAuthFile(filePath, provider);
export const deleteAuthFile = (fileId: string) =>
  backend.deleteAuthFile(fileId);
export const toggleAuthFile = (fileId: string, disabled: boolean) =>
  backend.toggleAuthFile(fileId, disabled);
export const downloadAuthFile = (fileId: string, filename: string) =>
  backend.downloadAuthFile(fileId, filename);
export const deleteAllAuthFiles = () => backend.deleteAllAuthFiles();

// Logs
export const getLogs = (lines?: number) => backend.getLogs(lines);
export const clearLogs = () => backend.clearLogs();

// Available Models
export const getAvailableModels = () => backend.getAvailableModels();

// Management API Settings
export const getMaxRetryInterval = () => backend.getMaxRetryInterval();
export const setMaxRetryInterval = (value: number) =>
  backend.setMaxRetryInterval(value);
export const getWebsocketAuth = () => backend.getWebsocketAuth();
export const setWebsocketAuth = (value: boolean) =>
  backend.setWebsocketAuth(value);
export const getPrioritizeModelMappings = () =>
  backend.getPrioritizeModelMappings();
export const setPrioritizeModelMappings = (value: boolean) =>
  backend.setPrioritizeModelMappings(value);
export const getOAuthExcludedModels = () => backend.getOAuthExcludedModels();
export const setOAuthExcludedModels = (provider: string, models: string[]) =>
  backend.setOAuthExcludedModels(provider, models);
export const deleteOAuthExcludedModels = (provider: string) =>
  backend.deleteOAuthExcludedModels(provider);
export const getConfigYaml = () => backend.getConfigYaml();
export const setConfigYaml = (yaml: string) => backend.setConfigYaml(yaml);
export const getRequestErrorLogs = () => backend.getRequestErrorLogs();
export const getRequestErrorLogContent = (filename: string) =>
  backend.getRequestErrorLogContent(filename);
