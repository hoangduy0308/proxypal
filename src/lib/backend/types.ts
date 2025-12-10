// Backend types - shared between Tauri and HTTP clients
// Re-export types from tauri.ts for compatibility

export interface ProxyStatus {
  running: boolean;
  port: number;
  endpoint: string;
}

export type Provider =
  | "claude"
  | "openai"
  | "gemini"
  | "qwen"
  | "iflow"
  | "vertex"
  | "antigravity";

export interface AuthStatus {
  claude: number;
  openai: number;
  gemini: number;
  qwen: number;
  iflow: number;
  vertex: number;
  antigravity: number;
}

export interface AmpModelMapping {
  from: string;
  to: string;
  enabled?: boolean;
}

export interface AmpModelSlot {
  id: string;
  name: string;
  fromModel: string;
  fromLabel: string;
}

export interface AmpOpenAIModel {
  name: string;
  alias: string;
}

export interface AmpOpenAIProvider {
  id: string;
  name: string;
  baseUrl: string;
  apiKey: string;
  models: AmpOpenAIModel[];
}

export interface CopilotConfig {
  enabled: boolean;
  port: number;
  accountType: string;
  githubToken: string;
  rateLimit?: number;
  rateLimitWait: boolean;
}

export interface CopilotStatus {
  running: boolean;
  port: number;
  endpoint: string;
  authenticated: boolean;
}

export interface CopilotApiDetection {
  installed: boolean;
  version?: string;
  copilotBin?: string;
  npxBin?: string;
  npmBin?: string;
  nodeBin?: string;
  nodeAvailable: boolean;
  checkedNodePaths: string[];
  checkedCopilotPaths: string[];
}

export interface CopilotApiInstallResult {
  success: boolean;
  message: string;
  version?: string;
}

export interface AppConfig {
  port: number;
  autoStart: boolean;
  launchAtLogin: boolean;
  debug: boolean;
  proxyUrl: string;
  requestRetry: number;
  quotaSwitchProject: boolean;
  quotaSwitchPreviewModel: boolean;
  usageStatsEnabled: boolean;
  requestLogging: boolean;
  loggingToFile: boolean;
  ampApiKey: string;
  ampModelMappings: AmpModelMapping[];
  ampOpenaiProvider?: AmpOpenAIProvider;
  ampOpenaiProviders: AmpOpenAIProvider[];
  ampRoutingMode: string;
  copilot: CopilotConfig;
}

export interface OAuthCallback {
  provider: Provider;
  code: string;
}

export interface RequestLog {
  id: string;
  timestamp: number;
  provider: string;
  model: string;
  method: string;
  path: string;
  status: number;
  durationMs: number;
  tokensIn?: number;
  tokensOut?: number;
}

export interface HealthStatus {
  status: "healthy" | "degraded" | "offline" | "unconfigured";
  latencyMs?: number;
  lastChecked: number;
}

export interface ProviderHealth {
  claude: HealthStatus;
  openai: HealthStatus;
  gemini: HealthStatus;
  qwen: HealthStatus;
  iflow: HealthStatus;
  vertex: HealthStatus;
  antigravity: HealthStatus;
}

export interface DetectedTool {
  id: string;
  name: string;
  installed: boolean;
  configPath?: string;
  canAutoConfigure: boolean;
}

export interface ToolSetupStep {
  title: string;
  description: string;
  copyable?: string;
}

export interface ToolSetupInfo {
  name: string;
  logo: string;
  canAutoConfigure: boolean;
  note?: string;
  steps: ToolSetupStep[];
  manualConfig?: string;
  endpoint?: string;
}

export interface AgentStatus {
  id: string;
  name: string;
  description: string;
  installed: boolean;
  configured: boolean;
  configType: "env" | "file" | "both" | "config";
  configPath?: string;
  logo: string;
  docsUrl: string;
}

export interface AgentConfigResult {
  success: boolean;
  configType: "env" | "file" | "both" | "config";
  configPath?: string;
  authPath?: string;
  shellConfig?: string;
  instructions: string;
  modelsConfigured?: number;
}

export interface TimeSeriesPoint {
  label: string;
  value: number;
}

export interface ModelUsage {
  model: string;
  requests: number;
  tokens: number;
}

export interface UsageStats {
  totalRequests: number;
  successCount: number;
  failureCount: number;
  totalTokens: number;
  inputTokens: number;
  outputTokens: number;
  requestsToday: number;
  tokensToday: number;
  models: ModelUsage[];
  requestsByDay: TimeSeriesPoint[];
  tokensByDay: TimeSeriesPoint[];
  requestsByHour: TimeSeriesPoint[];
  tokensByHour: TimeSeriesPoint[];
}

export interface RequestHistory {
  requests: RequestLog[];
  totalTokensIn: number;
  totalTokensOut: number;
  totalCostUsd: number;
}

export interface AgentTestResult {
  success: boolean;
  message: string;
  latencyMs?: number;
}

export interface ProviderTestResult {
  success: boolean;
  message: string;
  latencyMs?: number;
  modelsFound?: number;
}

export interface ModelMapping {
  name: string;
  alias?: string;
}

export interface GeminiApiKey {
  apiKey: string;
  baseUrl?: string;
  proxyUrl?: string;
  headers?: Record<string, string>;
  excludedModels?: string[];
}

export interface ClaudeApiKey {
  apiKey: string;
  baseUrl?: string;
  proxyUrl?: string;
  headers?: Record<string, string>;
  models?: ModelMapping[];
  excludedModels?: string[];
}

export interface CodexApiKey {
  apiKey: string;
  baseUrl?: string;
  proxyUrl?: string;
  headers?: Record<string, string>;
}

export interface OpenAICompatibleProvider {
  name: string;
  baseUrl: string;
  apiKeyEntries: Array<{
    apiKey: string;
    proxyUrl?: string;
  }>;
  models?: ModelMapping[];
  headers?: Record<string, string>;
}

export interface AuthFile {
  id: string;
  name: string;
  provider: string;
  label?: string;
  status: "ready" | "error" | "disabled";
  statusMessage?: string;
  disabled: boolean;
  unavailable: boolean;
  runtimeOnly: boolean;
  source?: "file" | "memory";
  path?: string;
  size?: number;
  modtime?: string;
  email?: string;
  accountType?: string;
  account?: string;
  createdAt?: string;
  updatedAt?: string;
  lastRefresh?: string;
  successCount?: number;
  failureCount?: number;
}

export interface LogEntry {
  timestamp: string;
  level: string;
  message: string;
}

export interface AvailableModel {
  id: string;
  ownedBy: string;
}

export interface GroupedModels {
  provider: string;
  models: string[];
}

export type OAuthExcludedModels = Record<string, string[]>;

// Unlisten function type for event listeners
export type UnlistenFn = () => void;

// Backend interface that both clients implement
export interface BackendClient {
  // Proxy management
  startProxy(): Promise<ProxyStatus>;
  stopProxy(): Promise<ProxyStatus>;
  getProxyStatus(): Promise<ProxyStatus>;

  // OAuth management
  openOAuth(provider: Provider): Promise<string>;
  pollOAuthStatus(oauthState: string): Promise<boolean>;
  completeOAuth(provider: Provider, code: string): Promise<AuthStatus>;
  disconnectProvider(provider: Provider): Promise<AuthStatus>;
  importVertexCredential(filePath: string): Promise<AuthStatus>;
  getAuthStatus(): Promise<AuthStatus>;
  refreshAuthStatus(): Promise<AuthStatus>;

  // Config
  getConfig(): Promise<AppConfig>;
  saveConfig(config: AppConfig): Promise<void>;

  // Event listeners (web mode returns no-op unlisten)
  onProxyStatusChanged(callback: (status: ProxyStatus) => void): Promise<UnlistenFn>;
  onAuthStatusChanged(callback: (status: AuthStatus) => void): Promise<UnlistenFn>;
  onOAuthCallback(callback: (data: OAuthCallback) => void): Promise<UnlistenFn>;
  onTrayToggleProxy(callback: (shouldStart: boolean) => void): Promise<UnlistenFn>;
  onCopilotStatusChanged(callback: (status: CopilotStatus) => void): Promise<UnlistenFn>;
  onCopilotAuthRequired(callback: (message: string) => void): Promise<UnlistenFn>;
  onRequestLog(callback: (log: RequestLog) => void): Promise<UnlistenFn>;

  // Copilot management
  getCopilotStatus(): Promise<CopilotStatus>;
  startCopilot(): Promise<CopilotStatus>;
  stopCopilot(): Promise<CopilotStatus>;
  checkCopilotHealth(): Promise<CopilotStatus>;
  detectCopilotApi(): Promise<CopilotApiDetection>;
  installCopilotApi(): Promise<CopilotApiInstallResult>;

  // System notifications (no-op in web mode)
  showSystemNotification(title: string, body?: string): Promise<void>;

  // Provider health
  checkProviderHealth(): Promise<ProviderHealth>;

  // AI Tool detection
  detectAiTools(): Promise<DetectedTool[]>;
  configureContinue(): Promise<string>;
  getToolSetupInfo(toolId: string): Promise<ToolSetupInfo>;

  // CLI Agents
  detectCliAgents(): Promise<AgentStatus[]>;
  configureCliAgent(agentId: string, models: AvailableModel[]): Promise<AgentConfigResult>;
  getShellProfilePath(): Promise<string>;
  appendToShellProfile(content: string): Promise<string>;

  // Usage Statistics
  getUsageStats(): Promise<UsageStats>;
  getRequestHistory(): Promise<RequestHistory>;
  addRequestToHistory(request: RequestLog): Promise<RequestHistory>;
  clearRequestHistory(): Promise<void>;
  syncUsageFromProxy(): Promise<RequestHistory>;

  // Agent testing
  testAgentConnection(agentId: string): Promise<AgentTestResult>;
  testOpenAIProvider(baseUrl: string, apiKey: string): Promise<ProviderTestResult>;

  // API Keys Management
  getGeminiApiKeys(): Promise<GeminiApiKey[]>;
  setGeminiApiKeys(keys: GeminiApiKey[]): Promise<void>;
  addGeminiApiKey(key: GeminiApiKey): Promise<void>;
  deleteGeminiApiKey(index: number): Promise<void>;

  getClaudeApiKeys(): Promise<ClaudeApiKey[]>;
  setClaudeApiKeys(keys: ClaudeApiKey[]): Promise<void>;
  addClaudeApiKey(key: ClaudeApiKey): Promise<void>;
  deleteClaudeApiKey(index: number): Promise<void>;

  getCodexApiKeys(): Promise<CodexApiKey[]>;
  setCodexApiKeys(keys: CodexApiKey[]): Promise<void>;
  addCodexApiKey(key: CodexApiKey): Promise<void>;
  deleteCodexApiKey(index: number): Promise<void>;

  getOpenAICompatibleProviders(): Promise<OpenAICompatibleProvider[]>;
  setOpenAICompatibleProviders(providers: OpenAICompatibleProvider[]): Promise<void>;
  addOpenAICompatibleProvider(provider: OpenAICompatibleProvider): Promise<void>;
  deleteOpenAICompatibleProvider(index: number): Promise<void>;

  // Auth Files Management
  getAuthFiles(): Promise<AuthFile[]>;
  uploadAuthFile(filePath: string, provider: string): Promise<void>;
  deleteAuthFile(fileId: string): Promise<void>;
  toggleAuthFile(fileId: string, disabled: boolean): Promise<void>;
  downloadAuthFile(fileId: string, filename: string): Promise<string>;
  deleteAllAuthFiles(): Promise<void>;

  // Logs
  getLogs(lines?: number): Promise<LogEntry[]>;
  clearLogs(): Promise<void>;

  // Available Models
  getAvailableModels(): Promise<AvailableModel[]>;

  // Management API Settings
  getMaxRetryInterval(): Promise<number>;
  setMaxRetryInterval(value: number): Promise<void>;
  getWebsocketAuth(): Promise<boolean>;
  setWebsocketAuth(value: boolean): Promise<void>;
  getPrioritizeModelMappings(): Promise<boolean>;
  setPrioritizeModelMappings(value: boolean): Promise<void>;
  getOAuthExcludedModels(): Promise<OAuthExcludedModels>;
  setOAuthExcludedModels(provider: string, models: string[]): Promise<void>;
  deleteOAuthExcludedModels(provider: string): Promise<void>;
  getConfigYaml(): Promise<string>;
  setConfigYaml(yaml: string): Promise<void>;
  getRequestErrorLogs(): Promise<string[]>;
  getRequestErrorLogContent(filename: string): Promise<string>;
}
