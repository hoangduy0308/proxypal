/**
 * ProxyPal Backend Adapter Types
 *
 * Unified type definitions that work for both Tauri desktop and HTTP server modes.
 * These types normalize the differences between Tauri's invoke-based API and the
 * HTTP server's REST API to provide a consistent interface for the frontend.
 */

// =============================================================================
// Core Proxy Types
// =============================================================================

/**
 * Proxy status information.
 * Tauri mode returns basic status; server mode includes additional metrics.
 */
export interface ProxyStatus {
  running: boolean;
  port: number;
  /** Process ID (server mode only) */
  pid?: number;
  /** Full endpoint URL */
  endpoint?: string;
  /** Uptime in seconds (server mode only) */
  uptimeSeconds?: number;
  /** Total requests processed (server mode only) */
  totalRequests?: number;
  /** List of active provider names (server mode only) */
  activeProviders?: string[];
}

// =============================================================================
// User Management (Server Mode Only)
// =============================================================================

/**
 * User account for API access.
 * @serverOnly Not available in Tauri desktop mode
 */
export interface User {
  id: number;
  name: string;
  apiKeyPrefix: string;
  quotaTokens: number | null;
  usedTokens: number;
  enabled: boolean;
  createdAt: string;
  lastUsedAt: string | null;
}

/**
 * Request payload for creating a new user.
 * @serverOnly
 */
export interface CreateUserRequest {
  name: string;
  quotaTokens?: number | null;
}

/**
 * Paginated list of users.
 * @serverOnly
 */
export interface UserListResponse {
  users: User[];
  total: number;
  page: number;
  limit: number;
}

/**
 * Response when creating a user - includes the full API key.
 * @serverOnly
 */
export interface CreateUserResponse extends User {
  /** Full API key (only returned once at creation) */
  apiKey?: string;
}

// =============================================================================
// Provider Types
// =============================================================================

/** Provider authentication type */
export type ProviderAuthType = "oauth" | "api_key";

/** Provider status */
export type ProviderStatusType = "active" | "inactive";

/**
 * Provider information - normalized from both Tauri and server responses.
 */
export interface ProviderInfo {
  name: string;
  type: ProviderAuthType;
  status: ProviderStatusType;
  /** Number of configured accounts */
  accounts: number;
  /** Available models for this provider */
  models: string[];
  lastUsedAt: string | null;
}

/**
 * Provider account details (server mode).
 * @serverOnly
 */
export interface ProviderAccount {
  id: number;
  email: string;
  status: "active" | "inactive" | "expired";
  addedAt: string;
  expiresAt?: string;
}

/**
 * Detailed provider info including accounts.
 * @serverOnly
 */
export interface ProviderDetails extends ProviderInfo {
  accountsList?: ProviderAccount[];
  settings?: {
    loadBalancing?: string;
    timeoutSeconds?: number;
  };
}

// =============================================================================
// Usage Statistics Types
// =============================================================================

/** Usage breakdown by provider or user */
export interface UsageBreakdown {
  requests: number;
  tokensInput: number;
  tokensOutput: number;
}

/**
 * Overall usage statistics.
 * Compatible with both Tauri and server responses.
 */
export interface UsageStats {
  period: string;
  totalRequests: number;
  totalTokensInput: number;
  totalTokensOutput: number;
  byProvider?: Record<string, UsageBreakdown>;
  /** Only available in server mode */
  byUser?: Record<string, UsageBreakdown>;
}

/**
 * User-specific usage statistics.
 * @serverOnly
 */
export interface UserUsage {
  userId: number;
  userName: string;
  period: string;
  totalRequests: number;
  totalTokensInput: number;
  totalTokensOutput: number;
  byProvider?: Record<string, UsageBreakdown>;
  byModel?: Record<string, { requests: number; tokens: number }>;
  daily?: { date: string; requests: number; tokens: number }[];
}

/**
 * Daily usage breakdown.
 */
export interface DailyUsage {
  days: number;
  data: {
    date: string;
    requests: number;
    tokensInput: number;
    tokensOutput: number;
  }[];
}

// =============================================================================
// Configuration Types
// =============================================================================

/**
 * Application configuration - normalized for both modes.
 * Server mode uses snake_case in API but we normalize to camelCase.
 */
export interface AppConfig {
  proxyPort: number;
  /** Admin server port (server mode only) */
  adminPort?: number;
  logLevel?: string;
  autoStartProxy?: boolean;
  /** Model mapping overrides (e.g., gpt-4 -> claude-3-opus) */
  modelMappings?: Record<string, string>;
  rateLimits?: {
    requestsPerMinute?: number;
    tokensPerDay?: number;
  };
}

/**
 * Response when saving configuration.
 */
export interface SaveConfigResponse {
  success: boolean;
  restartRequired: boolean;
}

// =============================================================================
// Authentication Types
// =============================================================================

/**
 * Authentication status for admin login.
 * Different shape between Tauri (provider counts) and server (session status).
 */
export interface AuthStatus {
  authenticated: boolean;
  expiresAt?: string;
}

/**
 * Tauri-specific auth status showing provider account counts.
 * @tauriOnly
 */
export interface TauriAuthStatus {
  claude: number;
  openai: number;
  gemini: number;
  qwen: number;
  iflow: number;
  vertex: number;
  antigravity: number;
}

// =============================================================================
// Request Logging Types
// =============================================================================

/**
 * Request log entry - compatible with both modes.
 */
export interface RequestLog {
  id: string | number;
  timestamp: string | number;
  provider: string;
  model: string;
  method?: string;
  path?: string;
  status: number | string;
  durationMs: number;
  tokensIn?: number;
  tokensOut?: number;
  /** Only in server mode */
  userId?: number;
  userName?: string;
}

/**
 * Paginated request logs response (server mode).
 * @serverOnly
 */
export interface RequestLogsResponse {
  logs: RequestLog[];
  total: number;
  limit: number;
  offset: number;
}

// =============================================================================
// Error Handling
// =============================================================================

/** Standard error codes from server API */
export type ErrorCode =
  | "UNAUTHORIZED"
  | "FORBIDDEN"
  | "NOT_FOUND"
  | "VALIDATION_ERROR"
  | "CONFLICT"
  | "QUOTA_EXCEEDED"
  | "RATE_LIMITED"
  | "PROVIDER_ERROR"
  | "INTERNAL_ERROR";

/**
 * Normalized backend error.
 */
export interface BackendError {
  message: string;
  code: ErrorCode | string;
  status?: number;
}

/**
 * Type guard to check if an error is a BackendError.
 */
export function isBackendError(error: unknown): error is BackendError {
  return (
    typeof error === "object" &&
    error !== null &&
    "message" in error &&
    "code" in error
  );
}

// =============================================================================
// Backend Client Interface
// =============================================================================

/** Function to unsubscribe from events */
export type UnlistenFn = () => void;

/**
 * Unified backend client interface.
 * Implementations: TauriClient (desktop) and HttpClient (server mode).
 */
export interface BackendClient {
  // -------------------------------------------------------------------------
  // Proxy Management (Both modes)
  // -------------------------------------------------------------------------

  /** Get current proxy status */
  getProxyStatus(): Promise<ProxyStatus>;

  /** Start the proxy server */
  startProxy(): Promise<ProxyStatus>;

  /** Stop the proxy server */
  stopProxy(): Promise<ProxyStatus>;

  /** Restart the proxy server */
  restartProxy(): Promise<ProxyStatus>;

  // -------------------------------------------------------------------------
  // User Management (Server mode only)
  // -------------------------------------------------------------------------

  /**
   * List all users with pagination.
   * @serverOnly Returns empty list in Tauri mode
   */
  listUsers(params?: { page?: number; limit?: number }): Promise<UserListResponse>;

  /**
   * Create a new user.
   * @serverOnly Throws in Tauri mode
   */
  createUser(data: CreateUserRequest): Promise<CreateUserResponse>;

  /**
   * Get user by ID.
   * @serverOnly Throws in Tauri mode
   */
  getUser(id: number): Promise<User>;

  /**
   * Update user details.
   * @serverOnly Throws in Tauri mode
   */
  updateUser(
    id: number,
    data: Partial<CreateUserRequest & { enabled: boolean }>
  ): Promise<User>;

  /**
   * Delete a user.
   * @serverOnly Throws in Tauri mode
   */
  deleteUser(id: number): Promise<void>;

  /**
   * Regenerate API key for a user.
   * @serverOnly Throws in Tauri mode
   */
  regenerateApiKey(id: number): Promise<{ apiKey: string; apiKeyPrefix: string }>;

  /**
   * Reset user's usage counter.
   * @serverOnly Throws in Tauri mode
   */
  resetUserUsage(id: number): Promise<{ previousUsedTokens: number }>;

  // -------------------------------------------------------------------------
  // Provider Management (Both modes)
  // -------------------------------------------------------------------------

  /** List all configured providers */
  listProviders(): Promise<{ providers: ProviderInfo[] }>;

  /** Get detailed provider info */
  getProvider(name: string): Promise<ProviderDetails>;

  /**
   * Remove a provider account.
   * @serverOnly Different signature in Tauri mode
   */
  removeProviderAccount(provider: string, accountId: number): Promise<void>;

  /**
   * Start OAuth flow for a provider.
   * In Tauri mode, opens browser. In server mode, returns redirect URL.
   */
  startOAuth(provider: string): Promise<void | { redirectUrl: string }>;

  // -------------------------------------------------------------------------
  // Usage Statistics (Both modes)
  // -------------------------------------------------------------------------

  /** Get overall usage statistics */
  getUsageStats(period?: string): Promise<UsageStats>;

  /**
   * Get usage for a specific user.
   * @serverOnly Throws in Tauri mode
   */
  getUserUsage(userId: number, period?: string): Promise<UserUsage>;

  /** Get daily usage breakdown */
  getDailyUsage(params?: {
    days?: number;
    userId?: number;
    provider?: string;
  }): Promise<DailyUsage>;

  // -------------------------------------------------------------------------
  // Configuration (Both modes)
  // -------------------------------------------------------------------------

  /** Get current configuration */
  getConfig(): Promise<AppConfig>;

  /** Save configuration changes */
  saveConfig(config: Partial<AppConfig>): Promise<SaveConfigResponse>;

  // -------------------------------------------------------------------------
  // Authentication (Server mode only)
  // -------------------------------------------------------------------------

  /**
   * Admin login.
   * @serverOnly Tauri mode doesn't require login
   */
  login(password: string): Promise<{ success: boolean }>;

  /**
   * Admin logout.
   * @serverOnly
   */
  logout(): Promise<void>;

  /**
   * Get authentication status.
   * In Tauri mode, returns { authenticated: true }.
   */
  getAuthStatus(): Promise<AuthStatus>;

  // -------------------------------------------------------------------------
  // Request Logs (Both modes)
  // -------------------------------------------------------------------------

  /**
   * Get request logs.
   * @serverOnly Different signature in Tauri mode
   */
  getRequestLogs?(params?: {
    limit?: number;
    offset?: number;
    userId?: number;
    provider?: string;
    status?: "success" | "error";
  }): Promise<RequestLogsResponse>;

  // -------------------------------------------------------------------------
  // Events (Tauri mode only)
  // -------------------------------------------------------------------------

  /**
   * Subscribe to proxy status changes.
   * @tauriOnly Returns no-op unsubscribe in server mode
   */
  onProxyStatusChanged(callback: (status: ProxyStatus) => void): Promise<UnlistenFn>;

  /**
   * Subscribe to auth status changes.
   * @tauriOnly
   */
  onAuthStatusChanged?(callback: (status: TauriAuthStatus) => void): Promise<UnlistenFn>;

  /**
   * Subscribe to request logs in real-time.
   * @tauriOnly
   */
  onRequestLog?(callback: (log: RequestLog) => void): Promise<UnlistenFn>;
}

// =============================================================================
// Utility Types
// =============================================================================

/** Backend mode */
export type BackendMode = "tauri" | "http";

/** Backend client factory function type */
export type BackendClientFactory = (mode: BackendMode, baseUrl?: string) => BackendClient;

/**
 * Convert snake_case keys to camelCase.
 * Useful for normalizing server API responses.
 */
export type SnakeToCamel<S extends string> = S extends `${infer T}_${infer U}`
  ? `${Lowercase<T>}${Capitalize<SnakeToCamel<U>>}`
  : Lowercase<S>;

/**
 * Convert all keys of an object from snake_case to camelCase.
 */
export type CamelCaseKeys<T> = {
  [K in keyof T as K extends string ? SnakeToCamel<K> : K]: T[K];
};
