/**
 * Type compatibility tests for ProxyPal backend adapter types.
 *
 * These tests verify that example payloads from API_DESIGN.md are assignable
 * to our type definitions, ensuring type compatibility at compile-time.
 */

import { describe, it, expect } from "vitest";
import type {
  ProxyStatus,
  User,
  UserListResponse,
  CreateUserResponse,
  ProviderInfo,
  ProviderDetails,
  UsageStats,
  UserUsage,
  DailyUsage,
  AppConfig,
  AuthStatus,
  RequestLog,
  BackendError,
} from "../types";
import { isBackendError } from "../types";

type AssertAssignable<T, U extends T> = U;

describe("ProxyStatus", () => {
  it("accepts Tauri-style response", () => {
    const tauriResponse = {
      running: true,
      port: 8317,
      endpoint: "http://localhost:8317",
    };

    const status: ProxyStatus = tauriResponse;
    expect(status.running).toBe(true);
    expect(status.port).toBe(8317);
  });

  it("accepts server-style response with all fields", () => {
    const serverResponse = {
      running: true,
      pid: 12345,
      port: 8317,
      uptimeSeconds: 7200,
      totalRequests: 1523,
      activeProviders: ["claude", "openai"],
    };

    const status: ProxyStatus = serverResponse;
    expect(status.pid).toBe(12345);
    expect(status.activeProviders).toContain("claude");
  });
});

describe("User", () => {
  it("accepts server API user response", () => {
    const serverUser = {
      id: 1,
      name: "john",
      apiKeyPrefix: "sk-john",
      quotaTokens: 1000000,
      usedTokens: 250000,
      enabled: true,
      createdAt: "2024-01-01T00:00:00Z",
      lastUsedAt: "2024-01-14T15:30:00Z",
    };

    const user: User = serverUser;
    expect(user.id).toBe(1);
    expect(user.quotaTokens).toBe(1000000);
  });

  it("accepts null quota (unlimited)", () => {
    const unlimitedUser = {
      id: 2,
      name: "jane",
      apiKeyPrefix: "sk-jane",
      quotaTokens: null,
      usedTokens: 500000,
      enabled: true,
      createdAt: "2024-01-05T00:00:00Z",
      lastUsedAt: null,
    };

    const user: User = unlimitedUser;
    expect(user.quotaTokens).toBeNull();
    expect(user.lastUsedAt).toBeNull();
  });
});

describe("UserListResponse", () => {
  it("accepts paginated user list", () => {
    const response = {
      users: [
        {
          id: 1,
          name: "john",
          apiKeyPrefix: "sk-john",
          quotaTokens: 1000000,
          usedTokens: 250000,
          enabled: true,
          createdAt: "2024-01-01T00:00:00Z",
          lastUsedAt: "2024-01-14T15:30:00Z",
        },
      ],
      total: 2,
      page: 1,
      limit: 50,
    };

    const list: UserListResponse = response;
    expect(list.users).toHaveLength(1);
    expect(list.total).toBe(2);
  });
});

describe("CreateUserResponse", () => {
  it("accepts user creation response with API key", () => {
    const response = {
      id: 3,
      name: "alice",
      apiKey: "sk-alice-xxxxxxxxxxxx",
      apiKeyPrefix: "sk-alice",
      quotaTokens: 500000,
      usedTokens: 0,
      enabled: true,
      createdAt: "2024-01-15T10:00:00Z",
      lastUsedAt: null,
    };

    const created: CreateUserResponse = response;
    expect(created.apiKey).toBeDefined();
    expect(created.usedTokens).toBe(0);
  });
});

describe("ProviderInfo", () => {
  it("accepts active provider", () => {
    const provider = {
      name: "claude",
      type: "oauth" as const,
      status: "active" as const,
      accounts: 2,
      models: ["claude-3-opus", "claude-3-sonnet", "claude-3-haiku"],
      lastUsedAt: "2024-01-14T16:00:00Z",
    };

    const info: ProviderInfo = provider;
    expect(info.type).toBe("oauth");
    expect(info.models).toHaveLength(3);
  });

  it("accepts inactive provider", () => {
    const provider = {
      name: "gemini",
      type: "oauth" as const,
      status: "inactive" as const,
      accounts: 0,
      models: [],
      lastUsedAt: null,
    };

    const info: ProviderInfo = provider;
    expect(info.status).toBe("inactive");
    expect(info.lastUsedAt).toBeNull();
  });
});

describe("ProviderDetails", () => {
  it("accepts detailed provider with accounts", () => {
    const details = {
      name: "claude",
      type: "oauth" as const,
      status: "active" as const,
      accounts: 2,
      models: ["claude-3-opus"],
      lastUsedAt: "2024-01-14T16:00:00Z",
      accountsList: [
        {
          id: 1,
          email: "user1@example.com",
          status: "active" as const,
          addedAt: "2024-01-01T00:00:00Z",
          expiresAt: "2024-02-01T00:00:00Z",
        },
      ],
      settings: {
        loadBalancing: "round_robin",
        timeoutSeconds: 120,
      },
    };

    const info: ProviderDetails = details;
    expect(info.accountsList).toHaveLength(1);
    expect(info.settings?.loadBalancing).toBe("round_robin");
  });
});

describe("UsageStats", () => {
  it("accepts overall usage stats", () => {
    const stats = {
      period: "month",
      totalRequests: 15230,
      totalTokensInput: 5000000,
      totalTokensOutput: 3000000,
      byProvider: {
        claude: {
          requests: 10000,
          tokensInput: 3500000,
          tokensOutput: 2000000,
        },
        openai: {
          requests: 5230,
          tokensInput: 1500000,
          tokensOutput: 1000000,
        },
      },
      byUser: {
        john: {
          requests: 8000,
          tokensInput: 2500000,
          tokensOutput: 1500000,
        },
      },
    };

    const usage: UsageStats = stats;
    expect(usage.totalRequests).toBe(15230);
    expect(usage.byProvider?.claude.requests).toBe(10000);
  });
});

describe("UserUsage", () => {
  it("accepts user-specific usage", () => {
    const usage = {
      userId: 1,
      userName: "john",
      period: "month",
      totalRequests: 8000,
      totalTokensInput: 2500000,
      totalTokensOutput: 1500000,
      byProvider: {
        claude: {
          requests: 5000,
          tokensInput: 1500000,
          tokensOutput: 1000000,
        },
      },
      byModel: {
        "claude-3-opus": { requests: 2000, tokens: 800000 },
      },
      daily: [
        { date: "2024-01-14", requests: 500, tokens: 150000 },
      ],
    };

    const userUsage: UserUsage = usage;
    expect(userUsage.userId).toBe(1);
    expect(userUsage.daily).toHaveLength(1);
  });
});

describe("DailyUsage", () => {
  it("accepts daily breakdown", () => {
    const daily = {
      days: 7,
      data: [
        {
          date: "2024-01-15",
          requests: 2100,
          tokensInput: 700000,
          tokensOutput: 400000,
        },
        {
          date: "2024-01-14",
          requests: 2300,
          tokensInput: 750000,
          tokensOutput: 450000,
        },
      ],
    };

    const usage: DailyUsage = daily;
    expect(usage.days).toBe(7);
    expect(usage.data).toHaveLength(2);
  });
});

describe("AppConfig", () => {
  it("accepts server config response", () => {
    const config = {
      proxyPort: 8317,
      adminPort: 3000,
      logLevel: "info",
      autoStartProxy: true,
      modelMappings: {
        "gpt-4": "claude-3-opus",
        "gpt-3.5-turbo": "claude-3-haiku",
      },
      rateLimits: {
        requestsPerMinute: 60,
        tokensPerDay: 1000000,
      },
    };

    const appConfig: AppConfig = config;
    expect(appConfig.proxyPort).toBe(8317);
    expect(appConfig.modelMappings?.["gpt-4"]).toBe("claude-3-opus");
  });

  it("accepts minimal config", () => {
    const config = {
      proxyPort: 8317,
    };

    const appConfig: AppConfig = config;
    expect(appConfig.adminPort).toBeUndefined();
  });
});

describe("AuthStatus", () => {
  it("accepts authenticated status", () => {
    const status = {
      authenticated: true,
      expiresAt: "2024-01-15T12:00:00Z",
    };

    const auth: AuthStatus = status;
    expect(auth.authenticated).toBe(true);
  });

  it("accepts unauthenticated status", () => {
    const status = {
      authenticated: false,
    };

    const auth: AuthStatus = status;
    expect(auth.authenticated).toBe(false);
    expect(auth.expiresAt).toBeUndefined();
  });
});

describe("RequestLog", () => {
  it("accepts server log entry", () => {
    const log = {
      id: 1523,
      timestamp: "2024-01-15T10:30:00Z",
      userId: 1,
      userName: "john",
      provider: "claude",
      model: "claude-3-opus",
      tokensIn: 500,
      tokensOut: 1200,
      durationMs: 2500,
      status: "success",
    };

    const entry: RequestLog = log;
    expect(entry.provider).toBe("claude");
    expect(entry.userName).toBe("john");
  });

  it("accepts Tauri log entry", () => {
    const log = {
      id: "abc-123",
      timestamp: 1705312200,
      provider: "claude",
      model: "claude-3-opus",
      method: "POST",
      path: "/v1/chat/completions",
      status: 200,
      durationMs: 2500,
      tokensIn: 500,
      tokensOut: 1200,
    };

    const entry: RequestLog = log;
    expect(entry.method).toBe("POST");
    expect(typeof entry.timestamp).toBe("number");
  });
});

describe("BackendError", () => {
  it("accepts error response", () => {
    const error = {
      message: "User not found",
      code: "NOT_FOUND",
      status: 404,
    };

    const backendError: BackendError = error;
    expect(backendError.code).toBe("NOT_FOUND");
  });

  it("isBackendError type guard works", () => {
    const error = {
      message: "Test error",
      code: "INTERNAL_ERROR",
    };

    expect(isBackendError(error)).toBe(true);
    expect(isBackendError(new Error("test"))).toBe(false);
    expect(isBackendError(null)).toBe(false);
    expect(isBackendError("string")).toBe(false);
  });
});

describe("Type compatibility between modes", () => {
  it("ProxyStatus is compatible between Tauri and server", () => {
    type TauriProxyStatus = {
      running: boolean;
      port: number;
      endpoint: string;
    };

    type ServerProxyStatus = {
      running: boolean;
      pid: number;
      port: number;
      uptimeSeconds: number;
      totalRequests: number;
      activeProviders: string[];
    };

    const _checkTauri: AssertAssignable<ProxyStatus, TauriProxyStatus> = {} as TauriProxyStatus;
    const _checkServer: AssertAssignable<ProxyStatus, ServerProxyStatus> = {} as ServerProxyStatus;

    expect(_checkTauri).toBeDefined();
    expect(_checkServer).toBeDefined();
  });
});
