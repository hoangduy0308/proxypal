/**
 * HTTP Backend Client for ProxyPal Server Mode
 *
 * Implements BackendClient interface using fetch() calls to REST API endpoints.
 * Handles CSRF tokens, error mapping, and snake_case to camelCase conversion.
 */

import type {
  BackendClient,
  ProxyStatus,
  User,
  UserListResponse,
  CreateUserRequest,
  CreateUserResponse,
  ProviderInfo,
  ProviderDetails,
  ProviderAccount,
  UsageStats,
  UserUsage,
  DailyUsage,
  AppConfig,
  SaveConfigResponse,
  AuthStatus,
  RequestLogsResponse,
  RequestLog,
  BackendError,
  UnlistenFn,
  TauriAuthStatus,
} from "./types";

// =============================================================================
// Helper Functions
// =============================================================================

/**
 * Get CSRF token from cookie
 */
function getCsrfToken(): string | null {
  const match = document.cookie.match(/csrf_token=([^;]+)/);
  return match ? match[1] : null;
}

/**
 * Build query string from params object
 */
function buildQueryString(params: Record<string, unknown>): string {
  const searchParams = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined && value !== null) {
      searchParams.append(key, String(value));
    }
  }
  const qs = searchParams.toString();
  return qs ? `?${qs}` : "";
}

/**
 * Fetch wrapper with error handling and CSRF token injection
 */
async function fetchJson<T>(
  path: string,
  options: RequestInit = {}
): Promise<T> {
  const isWriteMethod = ["POST", "PUT", "DELETE", "PATCH"].includes(
    options.method || "GET"
  );

  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    ...((options.headers as Record<string, string>) || {}),
  };

  if (isWriteMethod) {
    const csrf = getCsrfToken();
    if (csrf) headers["X-CSRF-Token"] = csrf;
  }

  const response = await fetch(path, {
    ...options,
    headers,
    credentials: "include",
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({}));
    throw {
      message: error.error || `HTTP ${response.status}`,
      code: error.code || "HTTP_ERROR",
      status: response.status,
    } as BackendError;
  }

  // Handle empty responses (204 No Content)
  if (response.status === 204) {
    return undefined as T;
  }

  return response.json();
}

// =============================================================================
// Response Mapping Helpers (snake_case -> camelCase)
// =============================================================================

function mapUser(u: Record<string, unknown>): User {
  return {
    id: u.id as number,
    name: u.name as string,
    apiKeyPrefix: u.api_key_prefix as string,
    quotaTokens: u.quota_tokens as number | null,
    usedTokens: u.used_tokens as number,
    enabled: u.enabled as boolean,
    createdAt: u.created_at as string,
    lastUsedAt: u.last_used_at as string | null,
  };
}

function mapCreateUserResponse(u: Record<string, unknown>): CreateUserResponse {
  return {
    ...mapUser(u),
    apiKey: u.api_key as string | undefined,
  };
}

function mapProxyStatus(res: Record<string, unknown>): ProxyStatus {
  return {
    running: res.running as boolean,
    port: res.port as number,
    pid: res.pid as number | undefined,
    uptimeSeconds: res.uptime_seconds as number | undefined,
    totalRequests: res.total_requests as number | undefined,
    activeProviders: res.active_providers as string[] | undefined,
  };
}

function mapProviderInfo(p: Record<string, unknown>): ProviderInfo {
  return {
    name: p.name as string,
    type: p.type as "oauth" | "api_key",
    status: p.status as "active" | "inactive",
    accounts: p.accounts as number,
    models: p.models as string[],
    lastUsedAt: p.last_used_at as string | null,
  };
}

function mapProviderAccount(a: Record<string, unknown>): ProviderAccount {
  return {
    id: a.id as number,
    email: a.email as string,
    status: a.status as "active" | "inactive" | "expired",
    addedAt: a.added_at as string,
    expiresAt: a.expires_at as string | undefined,
  };
}

function mapProviderDetails(res: Record<string, unknown>): ProviderDetails {
  const accounts = res.accounts as Record<string, unknown>[] | undefined;
  const settings = res.settings as Record<string, unknown> | undefined;

  return {
    name: res.name as string,
    type: res.type as "oauth" | "api_key",
    status: res.status as "active" | "inactive",
    accounts: Array.isArray(accounts) ? accounts.length : 0,
    models: (res.models as string[]) || [],
    lastUsedAt: res.last_used_at as string | null,
    accountsList: accounts?.map(mapProviderAccount),
    settings: settings
      ? {
          loadBalancing: settings.load_balancing as string | undefined,
          timeoutSeconds: settings.timeout_seconds as number | undefined,
        }
      : undefined,
  };
}

function mapUsageBreakdown(
  data: Record<string, Record<string, unknown>> | undefined
): Record<string, { requests: number; tokensInput: number; tokensOutput: number }> | undefined {
  if (!data) return undefined;
  const result: Record<string, { requests: number; tokensInput: number; tokensOutput: number }> = {};
  for (const [key, value] of Object.entries(data)) {
    result[key] = {
      requests: value.requests as number,
      tokensInput: value.tokens_input as number,
      tokensOutput: value.tokens_output as number,
    };
  }
  return result;
}

function mapUsageStats(res: Record<string, unknown>): UsageStats {
  return {
    period: res.period as string,
    totalRequests: res.total_requests as number,
    totalTokensInput: res.total_tokens_input as number,
    totalTokensOutput: res.total_tokens_output as number,
    byProvider: mapUsageBreakdown(res.by_provider as Record<string, Record<string, unknown>> | undefined),
    byUser: mapUsageBreakdown(res.by_user as Record<string, Record<string, unknown>> | undefined),
  };
}

function mapUserUsage(res: Record<string, unknown>): UserUsage {
  const byModel = res.by_model as Record<string, Record<string, unknown>> | undefined;
  const daily = res.daily as Record<string, unknown>[] | undefined;

  return {
    userId: res.user_id as number,
    userName: res.user_name as string,
    period: res.period as string,
    totalRequests: res.total_requests as number,
    totalTokensInput: res.total_tokens_input as number,
    totalTokensOutput: res.total_tokens_output as number,
    byProvider: mapUsageBreakdown(res.by_provider as Record<string, Record<string, unknown>> | undefined),
    byModel: byModel
      ? Object.fromEntries(
          Object.entries(byModel).map(([k, v]) => [
            k,
            { requests: v.requests as number, tokens: v.tokens as number },
          ])
        )
      : undefined,
    daily: daily?.map((d) => ({
      date: d.date as string,
      requests: d.requests as number,
      tokens: d.tokens as number,
    })),
  };
}

function mapDailyUsage(res: Record<string, unknown>): DailyUsage {
  const data = res.data as Record<string, unknown>[];
  return {
    days: res.days as number,
    data: data.map((d) => ({
      date: d.date as string,
      requests: d.requests as number,
      tokensInput: d.tokens_input as number,
      tokensOutput: d.tokens_output as number,
    })),
  };
}

function mapAppConfig(res: Record<string, unknown>): AppConfig {
  const rateLimits = res.rate_limits as Record<string, unknown> | undefined;
  return {
    proxyPort: res.proxy_port as number,
    adminPort: res.admin_port as number | undefined,
    logLevel: res.log_level as string | undefined,
    autoStartProxy: res.auto_start_proxy as boolean | undefined,
    modelMappings: res.model_mappings as Record<string, string> | undefined,
    rateLimits: rateLimits
      ? {
          requestsPerMinute: rateLimits.requests_per_minute as number | undefined,
          tokensPerDay: rateLimits.tokens_per_day as number | undefined,
        }
      : undefined,
  };
}

function mapConfigToSnakeCase(config: Partial<AppConfig>): Record<string, unknown> {
  const result: Record<string, unknown> = {};
  if (config.proxyPort !== undefined) result.proxy_port = config.proxyPort;
  if (config.adminPort !== undefined) result.admin_port = config.adminPort;
  if (config.logLevel !== undefined) result.log_level = config.logLevel;
  if (config.autoStartProxy !== undefined) result.auto_start_proxy = config.autoStartProxy;
  if (config.modelMappings !== undefined) result.model_mappings = config.modelMappings;
  if (config.rateLimits !== undefined) {
    result.rate_limits = {
      requests_per_minute: config.rateLimits.requestsPerMinute,
      tokens_per_day: config.rateLimits.tokensPerDay,
    };
  }
  return result;
}

// =============================================================================
// HTTP Backend Client Implementation
// =============================================================================

class HttpBackendClient implements BackendClient {
  // ---------------------------------------------------------------------------
  // Proxy Management
  // ---------------------------------------------------------------------------

  async getProxyStatus(): Promise<ProxyStatus> {
    const res = await fetchJson<Record<string, unknown>>("/api/proxy/status");
    return mapProxyStatus(res);
  }

  async startProxy(): Promise<ProxyStatus> {
    const res = await fetchJson<Record<string, unknown>>("/api/proxy/start", {
      method: "POST",
    });
    return {
      running: true,
      port: res.port as number,
      pid: res.pid as number | undefined,
    };
  }

  async stopProxy(): Promise<ProxyStatus> {
    await fetchJson("/api/proxy/stop", { method: "POST" });
    return { running: false, port: 8317 };
  }

  async restartProxy(): Promise<ProxyStatus> {
    const res = await fetchJson<Record<string, unknown>>("/api/proxy/restart", {
      method: "POST",
    });
    return {
      running: true,
      port: res.port as number,
      pid: res.pid as number | undefined,
    };
  }

  // ---------------------------------------------------------------------------
  // User Management
  // ---------------------------------------------------------------------------

  async listUsers(params?: { page?: number; limit?: number }): Promise<UserListResponse> {
    const qs = buildQueryString(params || {});
    const res = await fetchJson<Record<string, unknown>>(`/api/users${qs}`);
    const users = res.users as Record<string, unknown>[];
    return {
      users: users.map(mapUser),
      total: res.total as number,
      page: res.page as number,
      limit: res.limit as number,
    };
  }

  async createUser(data: CreateUserRequest): Promise<CreateUserResponse> {
    const res = await fetchJson<Record<string, unknown>>("/api/users", {
      method: "POST",
      body: JSON.stringify({
        name: data.name,
        quota_tokens: data.quotaTokens,
      }),
    });
    return mapCreateUserResponse(res);
  }

  async getUser(id: number): Promise<User> {
    const res = await fetchJson<Record<string, unknown>>(`/api/users/${id}`);
    return mapUser(res);
  }

  async updateUser(
    id: number,
    data: Partial<CreateUserRequest & { enabled: boolean }>
  ): Promise<User> {
    const body: Record<string, unknown> = {};
    if (data.name !== undefined) body.name = data.name;
    if (data.quotaTokens !== undefined) body.quota_tokens = data.quotaTokens;
    if (data.enabled !== undefined) body.enabled = data.enabled;

    const res = await fetchJson<Record<string, unknown>>(`/api/users/${id}`, {
      method: "PUT",
      body: JSON.stringify(body),
    });
    return mapUser(res);
  }

  async deleteUser(id: number): Promise<void> {
    await fetchJson(`/api/users/${id}`, { method: "DELETE" });
  }

  async regenerateApiKey(id: number): Promise<{ apiKey: string; apiKeyPrefix: string }> {
    const res = await fetchJson<Record<string, unknown>>(
      `/api/users/${id}/regenerate-key`,
      { method: "POST" }
    );
    return {
      apiKey: res.api_key as string,
      apiKeyPrefix: res.api_key_prefix as string,
    };
  }

  async resetUserUsage(id: number): Promise<{ previousUsedTokens: number }> {
    const res = await fetchJson<Record<string, unknown>>(
      `/api/users/${id}/reset-usage`,
      { method: "POST" }
    );
    return {
      previousUsedTokens: res.previous_used_tokens as number,
    };
  }

  // ---------------------------------------------------------------------------
  // Provider Management
  // ---------------------------------------------------------------------------

  async listProviders(): Promise<{ providers: ProviderInfo[] }> {
    const res = await fetchJson<Record<string, unknown>>("/api/providers");
    const providers = res.providers as Record<string, unknown>[];
    return {
      providers: providers.map(mapProviderInfo),
    };
  }

  async getProvider(name: string): Promise<ProviderDetails> {
    const res = await fetchJson<Record<string, unknown>>(`/api/providers/${name}`);
    return mapProviderDetails(res);
  }

  async removeProviderAccount(provider: string, accountId: number): Promise<void> {
    await fetchJson(`/api/providers/${provider}/accounts/${accountId}`, {
      method: "DELETE",
    });
  }

  async startOAuth(provider: string): Promise<void | { redirectUrl: string }> {
    return { redirectUrl: `/oauth/${provider}/start` };
  }

  // ---------------------------------------------------------------------------
  // Usage Statistics
  // ---------------------------------------------------------------------------

  async getUsageStats(period?: string): Promise<UsageStats> {
    const qs = buildQueryString({ period });
    const res = await fetchJson<Record<string, unknown>>(`/api/usage${qs}`);
    return mapUsageStats(res);
  }

  async getUserUsage(userId: number, period?: string): Promise<UserUsage> {
    const qs = buildQueryString({ period });
    const res = await fetchJson<Record<string, unknown>>(
      `/api/usage/users/${userId}${qs}`
    );
    return mapUserUsage(res);
  }

  async getDailyUsage(params?: {
    days?: number;
    userId?: number;
    provider?: string;
  }): Promise<DailyUsage> {
    const qs = buildQueryString({
      days: params?.days,
      user_id: params?.userId,
      provider: params?.provider,
    });
    const res = await fetchJson<Record<string, unknown>>(`/api/usage/daily${qs}`);
    return mapDailyUsage(res);
  }

  // ---------------------------------------------------------------------------
  // Configuration
  // ---------------------------------------------------------------------------

  async getConfig(): Promise<AppConfig> {
    const res = await fetchJson<Record<string, unknown>>("/api/config");
    return mapAppConfig(res);
  }

  async saveConfig(config: Partial<AppConfig>): Promise<SaveConfigResponse> {
    const res = await fetchJson<Record<string, unknown>>("/api/config", {
      method: "PUT",
      body: JSON.stringify(mapConfigToSnakeCase(config)),
    });
    return {
      success: res.success as boolean,
      restartRequired: res.restart_required as boolean,
    };
  }

  // ---------------------------------------------------------------------------
  // Authentication
  // ---------------------------------------------------------------------------

  async login(password: string): Promise<{ success: boolean }> {
    const res = await fetchJson<Record<string, unknown>>("/api/auth/login", {
      method: "POST",
      body: JSON.stringify({ password }),
    });
    return { success: res.success as boolean };
  }

  async logout(): Promise<void> {
    await fetchJson("/api/auth/logout", { method: "POST" });
  }

  async getAuthStatus(): Promise<AuthStatus> {
    const res = await fetchJson<Record<string, unknown>>("/api/auth/status");
    return {
      authenticated: res.authenticated as boolean,
      expiresAt: res.expires_at as string | undefined,
    };
  }

  // ---------------------------------------------------------------------------
  // Request Logs
  // ---------------------------------------------------------------------------

  async getRequestLogs(params?: {
    limit?: number;
    offset?: number;
    userId?: number;
    provider?: string;
    status?: "success" | "error";
  }): Promise<RequestLogsResponse> {
    const qs = buildQueryString({
      limit: params?.limit,
      offset: params?.offset,
      user_id: params?.userId,
      provider: params?.provider,
      status: params?.status,
    });
    const res = await fetchJson<Record<string, unknown>>(`/api/logs${qs}`);
    const logs = res.logs as Record<string, unknown>[];
    return {
      logs: logs.map((log) => ({
        id: log.id as string | number,
        timestamp: log.timestamp as string | number,
        provider: log.provider as string,
        model: log.model as string,
        method: log.method as string | undefined,
        path: log.path as string | undefined,
        status: log.status as number | string,
        durationMs: log.duration_ms as number,
        tokensIn: log.tokens_input as number | undefined,
        tokensOut: log.tokens_output as number | undefined,
        userId: log.user_id as number | undefined,
        userName: log.user_name as string | undefined,
      })),
      total: res.total as number,
      limit: res.limit as number,
      offset: res.offset as number,
    };
  }

  // ---------------------------------------------------------------------------
  // Events (No-op for HTTP mode - WebSocket support is Phase 6)
  // ---------------------------------------------------------------------------

  async onProxyStatusChanged(
    _callback: (status: ProxyStatus) => void
  ): Promise<UnlistenFn> {
    return () => {};
  }

  onAuthStatusChanged = async (
    _callback: (status: TauriAuthStatus) => void
  ): Promise<UnlistenFn> => {
    return () => {};
  };

  onRequestLog = async (
    _callback: (log: RequestLog) => void
  ): Promise<UnlistenFn> => {
    return () => {};
  };
}

// =============================================================================
// Export singleton instance
// =============================================================================

export const httpClient: BackendClient = new HttpBackendClient();

// Also export class for testing
export { HttpBackendClient, fetchJson, getCsrfToken, buildQueryString };
