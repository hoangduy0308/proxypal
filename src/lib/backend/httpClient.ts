// HTTP client implementation for web deployment
// Calls server API endpoints at /api/*

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
  UnlistenFn,
} from "./types";

class HttpError extends Error {
  constructor(
    public status: number,
    public code: string,
    message: string,
  ) {
    super(message);
    this.name = "HttpError";
  }
}

async function request<T>(
  method: string,
  path: string,
  body?: unknown,
): Promise<T> {
  const options: RequestInit = {
    method,
    credentials: "include",
    headers: {
      "Content-Type": "application/json",
    },
  };

  if (body !== undefined) {
    options.body = JSON.stringify(body);
  }

  const response = await fetch(path, options);

  if (!response.ok) {
    let errorData: { error?: string; code?: string } = {};
    try {
      errorData = await response.json();
    } catch {
      // ignore parse errors
    }
    throw new HttpError(
      response.status,
      errorData.code || "UNKNOWN_ERROR",
      errorData.error || response.statusText,
    );
  }

  const contentType = response.headers.get("content-type");
  if (contentType?.includes("application/json")) {
    return response.json();
  }

  return undefined as T;
}

function notSupported(feature: string): never {
  throw new Error(`${feature} is not supported in web mode`);
}

function noopUnlisten(): void {
  // No-op for web mode
}

export const httpClient: BackendClient = {
  // Proxy management
  async startProxy(): Promise<ProxyStatus> {
    const result = await request<{ success: boolean; port: number }>(
      "POST",
      "/api/proxy/start",
    );
    return {
      running: result.success,
      port: result.port,
      endpoint: `http://localhost:${result.port}`,
    };
  },

  async stopProxy(): Promise<ProxyStatus> {
    await request<{ success: boolean }>("POST", "/api/proxy/stop");
    return {
      running: false,
      port: 0,
      endpoint: "",
    };
  },

  async getProxyStatus(): Promise<ProxyStatus> {
    const result = await request<{
      running: boolean;
      port: number;
    }>("GET", "/api/proxy/status");
    return {
      running: result.running,
      port: result.port,
      endpoint: result.running ? `http://localhost:${result.port}` : "",
    };
  },

  // OAuth management
  async openOAuth(provider: Provider): Promise<string> {
    const result = await request<{ authUrl: string; state: string }>(
      "POST",
      `/api/providers/${provider}/oauth/start`,
    );
    // In web mode, we redirect the user to the auth URL
    window.open(result.authUrl, "_blank");
    return result.state;
  },

  async pollOAuthStatus(oauthState: string): Promise<boolean> {
    const result = await request<{ completed: boolean }>(
      "GET",
      `/api/oauth/status/${oauthState}`,
    );
    return result.completed;
  },

  async completeOAuth(provider: Provider, code: string): Promise<AuthStatus> {
    return request<AuthStatus>(
      "POST",
      `/api/providers/${provider}/oauth/complete`,
      { code },
    );
  },

  async disconnectProvider(provider: Provider): Promise<AuthStatus> {
    await request<{ success: boolean }>(
      "POST",
      `/api/providers/${provider}/disconnect`,
    );
    return this.getAuthStatus();
  },

  async importVertexCredential(_filePath: string): Promise<AuthStatus> {
    notSupported("Vertex credential import from file path");
  },

  async getAuthStatus(): Promise<AuthStatus> {
    const response = await request<{ authenticated: boolean }>(
      "GET",
      "/api/auth/status",
    );
    // Server returns different format, map to desktop format
    const defaultStatus: AuthStatus = {
      claude: 0,
      openai: 0,
      gemini: 0,
      qwen: 0,
      iflow: 0,
      vertex: 0,
      antigravity: 0,
    };

    if (!response.authenticated) {
      return defaultStatus;
    }

    // Get provider status from providers endpoint
    try {
      const providers = await request<{
        providers: Array<{ name: string; accountsCount: number }>;
      }>("GET", "/api/providers");
      for (const p of providers.providers) {
        const name = p.name.toLowerCase() as keyof AuthStatus;
        if (name in defaultStatus) {
          defaultStatus[name] = p.accountsCount;
        }
      }
    } catch {
      // Ignore errors
    }

    return defaultStatus;
  },

  async refreshAuthStatus(): Promise<AuthStatus> {
    return this.getAuthStatus();
  },

  // Config
  async getConfig(): Promise<AppConfig> {
    return request<AppConfig>("GET", "/api/config");
  },

  async saveConfig(config: AppConfig): Promise<void> {
    await request<{ success: boolean }>("PUT", "/api/config", config);
  },

  // Event listeners - in web mode these return no-op functions
  async onProxyStatusChanged(
    _callback: (status: ProxyStatus) => void,
  ): Promise<UnlistenFn> {
    // Web mode could use WebSocket/SSE for real-time updates
    return noopUnlisten;
  },

  async onAuthStatusChanged(
    _callback: (status: AuthStatus) => void,
  ): Promise<UnlistenFn> {
    return noopUnlisten;
  },

  async onOAuthCallback(
    _callback: (data: OAuthCallback) => void,
  ): Promise<UnlistenFn> {
    return noopUnlisten;
  },

  async onTrayToggleProxy(
    _callback: (shouldStart: boolean) => void,
  ): Promise<UnlistenFn> {
    return noopUnlisten;
  },

  async onCopilotStatusChanged(
    _callback: (status: CopilotStatus) => void,
  ): Promise<UnlistenFn> {
    return noopUnlisten;
  },

  async onCopilotAuthRequired(
    _callback: (message: string) => void,
  ): Promise<UnlistenFn> {
    return noopUnlisten;
  },

  async onRequestLog(
    _callback: (log: RequestLog) => void,
  ): Promise<UnlistenFn> {
    return noopUnlisten;
  },

  // Copilot management - not supported in web mode
  async getCopilotStatus(): Promise<CopilotStatus> {
    return {
      running: false,
      port: 0,
      endpoint: "",
      authenticated: false,
    };
  },

  async startCopilot(): Promise<CopilotStatus> {
    notSupported("Copilot management");
  },

  async stopCopilot(): Promise<CopilotStatus> {
    notSupported("Copilot management");
  },

  async checkCopilotHealth(): Promise<CopilotStatus> {
    return this.getCopilotStatus();
  },

  async detectCopilotApi(): Promise<CopilotApiDetection> {
    return {
      installed: false,
      nodeAvailable: false,
      checkedNodePaths: [],
      checkedCopilotPaths: [],
    };
  },

  async installCopilotApi(): Promise<CopilotApiInstallResult> {
    notSupported("Copilot API installation");
  },

  // System notifications - not supported in web mode
  async showSystemNotification(_title: string, _body?: string): Promise<void> {
    // Use browser notifications if available
    if ("Notification" in window && Notification.permission === "granted") {
      new Notification(_title, { body: _body });
    }
  },

  // Provider health
  async checkProviderHealth(): Promise<ProviderHealth> {
    try {
      const providers = await request<{
        providers: Array<{ name: string; status: string }>;
      }>("GET", "/api/providers");

      const health: ProviderHealth = {
        claude: { status: "unconfigured", lastChecked: Date.now() },
        openai: { status: "unconfigured", lastChecked: Date.now() },
        gemini: { status: "unconfigured", lastChecked: Date.now() },
        qwen: { status: "unconfigured", lastChecked: Date.now() },
        iflow: { status: "unconfigured", lastChecked: Date.now() },
        vertex: { status: "unconfigured", lastChecked: Date.now() },
        antigravity: { status: "unconfigured", lastChecked: Date.now() },
      };

      for (const p of providers.providers) {
        const name = p.name.toLowerCase() as keyof ProviderHealth;
        if (name in health) {
          health[name] = {
            status:
              p.status === "active"
                ? "healthy"
                : p.status === "error"
                  ? "offline"
                  : "unconfigured",
            lastChecked: Date.now(),
          };
        }
      }

      return health;
    } catch {
      return {
        claude: { status: "offline", lastChecked: Date.now() },
        openai: { status: "offline", lastChecked: Date.now() },
        gemini: { status: "offline", lastChecked: Date.now() },
        qwen: { status: "offline", lastChecked: Date.now() },
        iflow: { status: "offline", lastChecked: Date.now() },
        vertex: { status: "offline", lastChecked: Date.now() },
        antigravity: { status: "offline", lastChecked: Date.now() },
      };
    }
  },

  // AI Tool detection - not supported in web mode
  async detectAiTools(): Promise<DetectedTool[]> {
    return [];
  },

  async configureContinue(): Promise<string> {
    notSupported("AI tool configuration");
  },

  async getToolSetupInfo(_toolId: string): Promise<ToolSetupInfo> {
    notSupported("Tool setup info");
  },

  // CLI Agents - not supported in web mode
  async detectCliAgents(): Promise<AgentStatus[]> {
    return [];
  },

  async configureCliAgent(
    _agentId: string,
    _models: AvailableModel[],
  ): Promise<AgentConfigResult> {
    notSupported("CLI agent configuration");
  },

  async getShellProfilePath(): Promise<string> {
    notSupported("Shell profile path");
  },

  async appendToShellProfile(_content: string): Promise<string> {
    notSupported("Shell profile modification");
  },

  // Usage Statistics
  async getUsageStats(): Promise<UsageStats> {
    const result = await request<{
      totalRequests: number;
      totalTokensInput: number;
      totalTokensOutput: number;
    }>("GET", "/api/usage");

    return {
      totalRequests: result.totalRequests,
      successCount: result.totalRequests,
      failureCount: 0,
      totalTokens: result.totalTokensInput + result.totalTokensOutput,
      inputTokens: result.totalTokensInput,
      outputTokens: result.totalTokensOutput,
      requestsToday: 0,
      tokensToday: 0,
      models: [],
      requestsByDay: [],
      tokensByDay: [],
      requestsByHour: [],
      tokensByHour: [],
    };
  },

  async getRequestHistory(): Promise<RequestHistory> {
    const logs = await request<{ logs: RequestLog[] }>("GET", "/api/usage/logs");
    return {
      requests: logs.logs || [],
      totalTokensIn: 0,
      totalTokensOut: 0,
      totalCostUsd: 0,
    };
  },

  async addRequestToHistory(_request: RequestLog): Promise<RequestHistory> {
    // Server handles this automatically
    return this.getRequestHistory();
  },

  async clearRequestHistory(): Promise<void> {
    // Not typically supported via API
  },

  async syncUsageFromProxy(): Promise<RequestHistory> {
    return this.getRequestHistory();
  },

  // Agent testing
  async testAgentConnection(_agentId: string): Promise<AgentTestResult> {
    notSupported("Agent connection testing");
  },

  async testOpenAIProvider(
    _baseUrl: string,
    _apiKey: string,
  ): Promise<ProviderTestResult> {
    notSupported("Provider testing");
  },

  // API Keys Management - these may need server-side implementation
  async getGeminiApiKeys(): Promise<GeminiApiKey[]> {
    try {
      return await request<GeminiApiKey[]>("GET", "/api/config/gemini-keys");
    } catch {
      return [];
    }
  },

  async setGeminiApiKeys(keys: GeminiApiKey[]): Promise<void> {
    await request("PUT", "/api/config/gemini-keys", { keys });
  },

  async addGeminiApiKey(key: GeminiApiKey): Promise<void> {
    await request("POST", "/api/config/gemini-keys", key);
  },

  async deleteGeminiApiKey(index: number): Promise<void> {
    await request("DELETE", `/api/config/gemini-keys/${index}`);
  },

  async getClaudeApiKeys(): Promise<ClaudeApiKey[]> {
    try {
      return await request<ClaudeApiKey[]>("GET", "/api/config/claude-keys");
    } catch {
      return [];
    }
  },

  async setClaudeApiKeys(keys: ClaudeApiKey[]): Promise<void> {
    await request("PUT", "/api/config/claude-keys", { keys });
  },

  async addClaudeApiKey(key: ClaudeApiKey): Promise<void> {
    await request("POST", "/api/config/claude-keys", key);
  },

  async deleteClaudeApiKey(index: number): Promise<void> {
    await request("DELETE", `/api/config/claude-keys/${index}`);
  },

  async getCodexApiKeys(): Promise<CodexApiKey[]> {
    try {
      return await request<CodexApiKey[]>("GET", "/api/config/codex-keys");
    } catch {
      return [];
    }
  },

  async setCodexApiKeys(keys: CodexApiKey[]): Promise<void> {
    await request("PUT", "/api/config/codex-keys", { keys });
  },

  async addCodexApiKey(key: CodexApiKey): Promise<void> {
    await request("POST", "/api/config/codex-keys", key);
  },

  async deleteCodexApiKey(index: number): Promise<void> {
    await request("DELETE", `/api/config/codex-keys/${index}`);
  },

  async getOpenAICompatibleProviders(): Promise<OpenAICompatibleProvider[]> {
    try {
      return await request<OpenAICompatibleProvider[]>(
        "GET",
        "/api/config/openai-providers",
      );
    } catch {
      return [];
    }
  },

  async setOpenAICompatibleProviders(
    providers: OpenAICompatibleProvider[],
  ): Promise<void> {
    await request("PUT", "/api/config/openai-providers", { providers });
  },

  async addOpenAICompatibleProvider(
    provider: OpenAICompatibleProvider,
  ): Promise<void> {
    await request("POST", "/api/config/openai-providers", provider);
  },

  async deleteOpenAICompatibleProvider(index: number): Promise<void> {
    await request("DELETE", `/api/config/openai-providers/${index}`);
  },

  // Auth Files Management
  async getAuthFiles(): Promise<AuthFile[]> {
    try {
      const result = await request<{ files: AuthFile[] }>(
        "GET",
        "/api/auth/files",
      );
      return result.files || [];
    } catch {
      return [];
    }
  },

  async uploadAuthFile(_filePath: string, _provider: string): Promise<void> {
    notSupported("Auth file upload from file path");
  },

  async deleteAuthFile(fileId: string): Promise<void> {
    await request("DELETE", `/api/auth/files/${fileId}`);
  },

  async toggleAuthFile(fileId: string, disabled: boolean): Promise<void> {
    await request("PUT", `/api/auth/files/${fileId}`, { disabled });
  },

  async downloadAuthFile(_fileId: string, _filename: string): Promise<string> {
    notSupported("Auth file download");
  },

  async deleteAllAuthFiles(): Promise<void> {
    await request("DELETE", "/api/auth/files");
  },

  // Logs
  async getLogs(lines?: number): Promise<LogEntry[]> {
    const query = lines ? `?lines=${lines}` : "";
    const result = await request<{ logs: LogEntry[] }>(
      "GET",
      `/api/logs${query}`,
    );
    return result.logs || [];
  },

  async clearLogs(): Promise<void> {
    await request("DELETE", "/api/logs");
  },

  // Available Models
  async getAvailableModels(): Promise<AvailableModel[]> {
    try {
      const result = await request<{ data: Array<{ id: string; owned_by: string }> }>(
        "GET",
        "/v1/models",
      );
      return (result.data || []).map((m) => ({
        id: m.id,
        ownedBy: m.owned_by,
      }));
    } catch {
      return [];
    }
  },

  // Management API Settings
  async getMaxRetryInterval(): Promise<number> {
    const config = await this.getConfig();
    return config.requestRetry || 3;
  },

  async setMaxRetryInterval(value: number): Promise<void> {
    const config = await this.getConfig();
    config.requestRetry = value;
    await this.saveConfig(config);
  },

  async getWebsocketAuth(): Promise<boolean> {
    return false;
  },

  async setWebsocketAuth(_value: boolean): Promise<void> {
    // Not applicable in web mode
  },

  async getPrioritizeModelMappings(): Promise<boolean> {
    return false;
  },

  async setPrioritizeModelMappings(_value: boolean): Promise<void> {
    // Not applicable in web mode
  },

  async getOAuthExcludedModels(): Promise<OAuthExcludedModels> {
    return {};
  },

  async setOAuthExcludedModels(
    _provider: string,
    _models: string[],
  ): Promise<void> {
    // Not applicable in web mode
  },

  async deleteOAuthExcludedModels(_provider: string): Promise<void> {
    // Not applicable in web mode
  },

  async getConfigYaml(): Promise<string> {
    notSupported("Raw YAML config access");
  },

  async setConfigYaml(_yaml: string): Promise<void> {
    notSupported("Raw YAML config modification");
  },

  async getRequestErrorLogs(): Promise<string[]> {
    return [];
  },

  async getRequestErrorLogContent(_filename: string): Promise<string> {
    notSupported("Error log content access");
  },
};
