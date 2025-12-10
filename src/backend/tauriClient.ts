/**
 * Tauri Backend Client Implementation
 *
 * Implements BackendClient interface by delegating to existing Tauri functions.
 * Server-only methods throw BackendError with code 'UNSUPPORTED_IN_TAURI'.
 */

import type {
  BackendClient,
  BackendError,
  ProxyStatus,
  AppConfig,
  SaveConfigResponse,
  UsageStats,
  DailyUsage,
  UserListResponse,
  CreateUserRequest,
  CreateUserResponse,
  User,
  UserUsage,
  ProviderInfo,
  ProviderDetails,
  AuthStatus,
  TauriAuthStatus,
  RequestLog,
  RequestLogsResponse,
  UnlistenFn,
} from "./types";
import {
  startProxy as tauriStartProxy,
  stopProxy as tauriStopProxy,
  getProxyStatus as tauriGetProxyStatus,
  getConfig as tauriGetConfig,
  saveConfig as tauriSaveConfig,
  getUsageStats as tauriGetUsageStats,
  getAuthStatus as tauriGetAuthStatus,
  openOAuth as tauriOpenOAuth,
  disconnectProvider as tauriDisconnectProvider,
  onProxyStatusChanged as tauriOnProxyStatusChanged,
  onAuthStatusChanged as tauriOnAuthStatusChanged,
  onRequestLog as tauriOnRequestLog,
  type ProxyStatus as TauriProxyStatus,
  type AppConfig as TauriAppConfig,
  type UsageStats as TauriUsageStats,
  type Provider as TauriProvider,
  type RequestLog as TauriRequestLog,
} from "../lib/tauri";

function createUnsupportedError(method: string): BackendError {
  return {
    message: `${method} is only available in server mode`,
    code: "UNSUPPORTED_IN_TAURI",
  };
}

function mapProxyStatus(status: TauriProxyStatus): ProxyStatus {
  return {
    running: status.running,
    port: status.port,
    endpoint: status.endpoint,
  };
}

function mapAppConfig(config: TauriAppConfig): AppConfig {
  return {
    proxyPort: config.port,
    autoStartProxy: config.autoStart,
    modelMappings: config.ampModelMappings?.reduce(
      (acc, m) => {
        if (m.enabled !== false) {
          acc[m.from] = m.to;
        }
        return acc;
      },
      {} as Record<string, string>
    ),
  };
}

function mapUsageStats(stats: TauriUsageStats): UsageStats {
  const byProvider: Record<string, { requests: number; tokensInput: number; tokensOutput: number }> = {};
  for (const model of stats.models) {
    const provider = getProviderFromModel(model.model);
    if (!byProvider[provider]) {
      byProvider[provider] = { requests: 0, tokensInput: 0, tokensOutput: 0 };
    }
    byProvider[provider].requests += model.requests;
    byProvider[provider].tokensInput += model.tokens;
    byProvider[provider].tokensOutput += 0;
  }

  return {
    period: "all",
    totalRequests: stats.totalRequests,
    totalTokensInput: stats.inputTokens,
    totalTokensOutput: stats.outputTokens,
    byProvider,
  };
}

function getProviderFromModel(model: string): string {
  const lower = model.toLowerCase();
  if (lower.includes("claude")) return "claude";
  if (lower.includes("gpt") || lower.includes("o1")) return "openai";
  if (lower.includes("gemini")) return "gemini";
  if (lower.includes("qwen")) return "qwen";
  return "unknown";
}

function mapTauriRequestLog(log: TauriRequestLog): RequestLog {
  return {
    id: log.id,
    timestamp: log.timestamp,
    provider: log.provider,
    model: log.model,
    method: log.method,
    path: log.path,
    status: log.status,
    durationMs: log.durationMs,
    tokensIn: log.tokensIn,
    tokensOut: log.tokensOut,
  };
}

class TauriBackendClient implements BackendClient {
  // -------------------------------------------------------------------------
  // Proxy Management
  // -------------------------------------------------------------------------

  async getProxyStatus(): Promise<ProxyStatus> {
    const status = await tauriGetProxyStatus();
    return mapProxyStatus(status);
  }

  async startProxy(): Promise<ProxyStatus> {
    const status = await tauriStartProxy();
    return mapProxyStatus(status);
  }

  async stopProxy(): Promise<ProxyStatus> {
    const status = await tauriStopProxy();
    return mapProxyStatus(status);
  }

  async restartProxy(): Promise<ProxyStatus> {
    await tauriStopProxy();
    const status = await tauriStartProxy();
    return mapProxyStatus(status);
  }

  // -------------------------------------------------------------------------
  // User Management (Server mode only)
  // -------------------------------------------------------------------------

  async listUsers(_params?: { page?: number; limit?: number }): Promise<UserListResponse> {
    throw createUnsupportedError("listUsers");
  }

  async createUser(_data: CreateUserRequest): Promise<CreateUserResponse> {
    throw createUnsupportedError("createUser");
  }

  async getUser(_id: number): Promise<User> {
    throw createUnsupportedError("getUser");
  }

  async updateUser(
    _id: number,
    _data: Partial<CreateUserRequest & { enabled: boolean }>
  ): Promise<User> {
    throw createUnsupportedError("updateUser");
  }

  async deleteUser(_id: number): Promise<void> {
    throw createUnsupportedError("deleteUser");
  }

  async regenerateApiKey(_id: number): Promise<{ apiKey: string; apiKeyPrefix: string }> {
    throw createUnsupportedError("regenerateApiKey");
  }

  async resetUserUsage(_id: number): Promise<{ previousUsedTokens: number }> {
    throw createUnsupportedError("resetUserUsage");
  }

  // -------------------------------------------------------------------------
  // Provider Management
  // -------------------------------------------------------------------------

  async listProviders(): Promise<{ providers: ProviderInfo[] }> {
    const authStatus = await tauriGetAuthStatus();
    const providers: ProviderInfo[] = [];

    const providerNames: TauriProvider[] = [
      "claude",
      "openai",
      "gemini",
      "qwen",
      "iflow",
      "vertex",
      "antigravity",
    ];

    for (const name of providerNames) {
      const accountCount = authStatus[name] ?? 0;
      providers.push({
        name,
        type: name === "vertex" ? "api_key" : "oauth",
        status: accountCount > 0 ? "active" : "inactive",
        accounts: accountCount,
        models: [],
        lastUsedAt: null,
      });
    }

    return { providers };
  }

  async getProvider(name: string): Promise<ProviderDetails> {
    const authStatus = await tauriGetAuthStatus();
    const providerName = name as TauriProvider;
    const accountCount = authStatus[providerName] ?? 0;

    return {
      name,
      type: name === "vertex" ? "api_key" : "oauth",
      status: accountCount > 0 ? "active" : "inactive",
      accounts: accountCount,
      models: [],
      lastUsedAt: null,
    };
  }

  async removeProviderAccount(provider: string, _accountId: number): Promise<void> {
    await tauriDisconnectProvider(provider as TauriProvider);
  }

  async startOAuth(provider: string): Promise<void | { redirectUrl: string }> {
    await tauriOpenOAuth(provider as TauriProvider);
  }

  // -------------------------------------------------------------------------
  // Usage Statistics
  // -------------------------------------------------------------------------

  async getUsageStats(_period?: string): Promise<UsageStats> {
    const stats = await tauriGetUsageStats();
    return mapUsageStats(stats);
  }

  async getUserUsage(_userId: number, _period?: string): Promise<UserUsage> {
    throw createUnsupportedError("getUserUsage");
  }

  async getDailyUsage(params?: {
    days?: number;
    userId?: number;
    provider?: string;
  }): Promise<DailyUsage> {
    const stats = await tauriGetUsageStats();
    const days = params?.days ?? 7;

    const data = stats.requestsByDay.slice(-days).map((point) => ({
      date: point.label,
      requests: point.value,
      tokensInput: 0,
      tokensOutput: 0,
    }));

    const tokensByDayMap = new Map(
      stats.tokensByDay.map((p) => [p.label, p.value])
    );

    for (const d of data) {
      const tokens = tokensByDayMap.get(d.date) ?? 0;
      d.tokensInput = tokens;
    }

    return {
      days,
      data,
    };
  }

  // -------------------------------------------------------------------------
  // Configuration
  // -------------------------------------------------------------------------

  async getConfig(): Promise<AppConfig> {
    const config = await tauriGetConfig();
    return mapAppConfig(config);
  }

  async saveConfig(config: Partial<AppConfig>): Promise<SaveConfigResponse> {
    const currentConfig = await tauriGetConfig();

    const updatedConfig: TauriAppConfig = {
      ...currentConfig,
    };

    if (config.proxyPort !== undefined) {
      updatedConfig.port = config.proxyPort;
    }
    if (config.autoStartProxy !== undefined) {
      updatedConfig.autoStart = config.autoStartProxy;
    }

    await tauriSaveConfig(updatedConfig);

    return {
      success: true,
      restartRequired: config.proxyPort !== undefined && config.proxyPort !== currentConfig.port,
    };
  }

  // -------------------------------------------------------------------------
  // Authentication
  // -------------------------------------------------------------------------

  async login(_password: string): Promise<{ success: boolean }> {
    return { success: true };
  }

  async logout(): Promise<void> {
    // No-op in Tauri mode
  }

  async getAuthStatus(): Promise<AuthStatus> {
    return { authenticated: true };
  }

  // -------------------------------------------------------------------------
  // Request Logs
  // -------------------------------------------------------------------------

  async getRequestLogs(_params?: {
    limit?: number;
    offset?: number;
    userId?: number;
    provider?: string;
    status?: "success" | "error";
  }): Promise<RequestLogsResponse> {
    throw createUnsupportedError("getRequestLogs");
  }

  // -------------------------------------------------------------------------
  // Events
  // -------------------------------------------------------------------------

  async onProxyStatusChanged(callback: (status: ProxyStatus) => void): Promise<UnlistenFn> {
    return tauriOnProxyStatusChanged((status) => {
      callback(mapProxyStatus(status));
    });
  }

  async onAuthStatusChanged(callback: (status: TauriAuthStatus) => void): Promise<UnlistenFn> {
    return tauriOnAuthStatusChanged(callback);
  }

  async onRequestLog(callback: (log: RequestLog) => void): Promise<UnlistenFn> {
    return tauriOnRequestLog((log) => {
      callback(mapTauriRequestLog(log));
    });
  }
}

export const tauriClient: BackendClient = new TauriBackendClient();
