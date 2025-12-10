/**
 * Tests for HTTP Backend Client
 */

import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { HttpBackendClient, fetchJson, getCsrfToken, buildQueryString } from "../httpClient";
import type { BackendError } from "../types";

const mockFetch = vi.fn();
(globalThis as unknown as { fetch: typeof fetch }).fetch = mockFetch;

describe("httpClient", () => {
  beforeEach(() => {
    mockFetch.mockReset();
    // Clear cookies
    Object.defineProperty(document, "cookie", {
      writable: true,
      value: "",
    });
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  // ===========================================================================
  // Helper Function Tests
  // ===========================================================================

  describe("getCsrfToken", () => {
    it("returns null when no CSRF cookie exists", () => {
      document.cookie = "";
      expect(getCsrfToken()).toBeNull();
    });

    it("extracts CSRF token from cookie", () => {
      document.cookie = "csrf_token=abc123; other=value";
      expect(getCsrfToken()).toBe("abc123");
    });

    it("handles CSRF token with special characters", () => {
      document.cookie = "csrf_token=abc-123_XYZ";
      expect(getCsrfToken()).toBe("abc-123_XYZ");
    });
  });

  describe("buildQueryString", () => {
    it("returns empty string for empty params", () => {
      expect(buildQueryString({})).toBe("");
    });

    it("builds query string from params", () => {
      expect(buildQueryString({ page: 1, limit: 50 })).toBe("?page=1&limit=50");
    });

    it("skips undefined and null values", () => {
      expect(buildQueryString({ page: 1, limit: undefined, foo: null })).toBe(
        "?page=1"
      );
    });
  });

  // ===========================================================================
  // CSRF Token Handling Tests
  // ===========================================================================

  describe("CSRF token handling", () => {
    it("adds CSRF header for POST requests", async () => {
      document.cookie = "csrf_token=test-csrf-token";
      mockFetch.mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ success: true }),
      });

      await fetchJson("/api/test", { method: "POST" });

      expect(mockFetch).toHaveBeenCalledWith(
        "/api/test",
        expect.objectContaining({
          headers: expect.objectContaining({
            "X-CSRF-Token": "test-csrf-token",
          }),
        })
      );
    });

    it("adds CSRF header for PUT requests", async () => {
      document.cookie = "csrf_token=csrf-put";
      mockFetch.mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ success: true }),
      });

      await fetchJson("/api/test", { method: "PUT" });

      expect(mockFetch).toHaveBeenCalledWith(
        "/api/test",
        expect.objectContaining({
          headers: expect.objectContaining({
            "X-CSRF-Token": "csrf-put",
          }),
        })
      );
    });

    it("adds CSRF header for DELETE requests", async () => {
      document.cookie = "csrf_token=csrf-delete";
      mockFetch.mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ success: true }),
      });

      await fetchJson("/api/test", { method: "DELETE" });

      expect(mockFetch).toHaveBeenCalledWith(
        "/api/test",
        expect.objectContaining({
          headers: expect.objectContaining({
            "X-CSRF-Token": "csrf-delete",
          }),
        })
      );
    });

    it("does not add CSRF header for GET requests", async () => {
      document.cookie = "csrf_token=should-not-appear";
      mockFetch.mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: () => Promise.resolve({}),
      });

      await fetchJson("/api/test");

      expect(mockFetch).toHaveBeenCalledWith(
        "/api/test",
        expect.objectContaining({
          headers: expect.not.objectContaining({
            "X-CSRF-Token": expect.any(String),
          }),
        })
      );
    });

    it("includes credentials in all requests", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: () => Promise.resolve({}),
      });

      await fetchJson("/api/test");

      expect(mockFetch).toHaveBeenCalledWith(
        "/api/test",
        expect.objectContaining({
          credentials: "include",
        })
      );
    });
  });

  // ===========================================================================
  // Error Handling Tests
  // ===========================================================================

  describe("error handling", () => {
    it("maps HTTP error to BackendError", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 401,
        json: () => Promise.resolve({ error: "Unauthorized", code: "UNAUTHORIZED" }),
      });

      await expect(fetchJson("/api/test")).rejects.toMatchObject({
        message: "Unauthorized",
        code: "UNAUTHORIZED",
        status: 401,
      } as BackendError);
    });

    it("handles error response without JSON body", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 500,
        json: () => Promise.reject(new Error("Invalid JSON")),
      });

      await expect(fetchJson("/api/test")).rejects.toMatchObject({
        message: "HTTP 500",
        code: "HTTP_ERROR",
        status: 500,
      });
    });

    it("handles 404 Not Found error", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 404,
        json: () => Promise.resolve({ error: "User not found", code: "NOT_FOUND" }),
      });

      await expect(fetchJson("/api/users/999")).rejects.toMatchObject({
        message: "User not found",
        code: "NOT_FOUND",
        status: 404,
      });
    });

    it("handles validation error", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 400,
        json: () =>
          Promise.resolve({ error: "Name is required", code: "VALIDATION_ERROR" }),
      });

      await expect(fetchJson("/api/users", { method: "POST" })).rejects.toMatchObject({
        message: "Name is required",
        code: "VALIDATION_ERROR",
        status: 400,
      });
    });
  });

  // ===========================================================================
  // Response Mapping Tests (snake_case -> camelCase)
  // ===========================================================================

  describe("response mapping", () => {
    let client: HttpBackendClient;

    beforeEach(() => {
      client = new HttpBackendClient();
    });

    describe("getProxyStatus", () => {
      it("maps snake_case response to camelCase", async () => {
        mockFetch.mockResolvedValueOnce({
          ok: true,
          status: 200,
          json: () =>
            Promise.resolve({
              running: true,
              port: 8317,
              pid: 12345,
              uptime_seconds: 3600,
              total_requests: 1000,
              active_providers: ["claude", "openai"],
            }),
        });

        const result = await client.getProxyStatus();

        expect(result).toEqual({
          running: true,
          port: 8317,
          pid: 12345,
          uptimeSeconds: 3600,
          totalRequests: 1000,
          activeProviders: ["claude", "openai"],
        });
      });
    });

    describe("listUsers", () => {
      it("maps user list response correctly", async () => {
        mockFetch.mockResolvedValueOnce({
          ok: true,
          status: 200,
          json: () =>
            Promise.resolve({
              users: [
                {
                  id: 1,
                  name: "john",
                  api_key_prefix: "sk-john",
                  quota_tokens: 1000000,
                  used_tokens: 250000,
                  enabled: true,
                  created_at: "2024-01-01T00:00:00Z",
                  last_used_at: "2024-01-14T15:30:00Z",
                },
              ],
              total: 1,
              page: 1,
              limit: 50,
            }),
        });

        const result = await client.listUsers();

        expect(result.users[0]).toEqual({
          id: 1,
          name: "john",
          apiKeyPrefix: "sk-john",
          quotaTokens: 1000000,
          usedTokens: 250000,
          enabled: true,
          createdAt: "2024-01-01T00:00:00Z",
          lastUsedAt: "2024-01-14T15:30:00Z",
        });
      });

      it("handles null quota_tokens", async () => {
        mockFetch.mockResolvedValueOnce({
          ok: true,
          status: 200,
          json: () =>
            Promise.resolve({
              users: [
                {
                  id: 2,
                  name: "jane",
                  api_key_prefix: "sk-jane",
                  quota_tokens: null,
                  used_tokens: 500000,
                  enabled: true,
                  created_at: "2024-01-05T00:00:00Z",
                  last_used_at: null,
                },
              ],
              total: 1,
              page: 1,
              limit: 50,
            }),
        });

        const result = await client.listUsers();

        expect(result.users[0].quotaTokens).toBeNull();
        expect(result.users[0].lastUsedAt).toBeNull();
      });
    });

    describe("createUser", () => {
      it("maps create user response with apiKey", async () => {
        mockFetch.mockResolvedValueOnce({
          ok: true,
          status: 201,
          json: () =>
            Promise.resolve({
              id: 3,
              name: "alice",
              api_key: "sk-alice-abc123xyz",
              api_key_prefix: "sk-alice",
              quota_tokens: 500000,
              used_tokens: 0,
              enabled: true,
              created_at: "2024-01-15T10:00:00Z",
              last_used_at: null,
            }),
        });

        const result = await client.createUser({ name: "alice", quotaTokens: 500000 });

        expect(result.apiKey).toBe("sk-alice-abc123xyz");
        expect(result.apiKeyPrefix).toBe("sk-alice");
      });
    });

    describe("getProvider", () => {
      it("maps provider details with accounts", async () => {
        mockFetch.mockResolvedValueOnce({
          ok: true,
          status: 200,
          json: () =>
            Promise.resolve({
              name: "claude",
              type: "oauth",
              status: "active",
              models: ["claude-3-opus", "claude-3-sonnet"],
              last_used_at: "2024-01-14T16:00:00Z",
              accounts: [
                {
                  id: 1,
                  email: "user1@example.com",
                  status: "active",
                  added_at: "2024-01-01T00:00:00Z",
                  expires_at: "2024-02-01T00:00:00Z",
                },
              ],
              settings: {
                load_balancing: "round_robin",
                timeout_seconds: 120,
              },
            }),
        });

        const result = await client.getProvider("claude");

        expect(result.accountsList?.[0]).toEqual({
          id: 1,
          email: "user1@example.com",
          status: "active",
          addedAt: "2024-01-01T00:00:00Z",
          expiresAt: "2024-02-01T00:00:00Z",
        });
        expect(result.settings).toEqual({
          loadBalancing: "round_robin",
          timeoutSeconds: 120,
        });
      });
    });

    describe("getUsageStats", () => {
      it("maps usage stats with byProvider and byUser", async () => {
        mockFetch.mockResolvedValueOnce({
          ok: true,
          status: 200,
          json: () =>
            Promise.resolve({
              period: "month",
              total_requests: 15230,
              total_tokens_input: 5000000,
              total_tokens_output: 3000000,
              by_provider: {
                claude: {
                  requests: 10000,
                  tokens_input: 3500000,
                  tokens_output: 2000000,
                },
              },
              by_user: {
                john: {
                  requests: 8000,
                  tokens_input: 2500000,
                  tokens_output: 1500000,
                },
              },
            }),
        });

        const result = await client.getUsageStats("month");

        expect(result.totalRequests).toBe(15230);
        expect(result.totalTokensInput).toBe(5000000);
        expect(result.byProvider?.claude).toEqual({
          requests: 10000,
          tokensInput: 3500000,
          tokensOutput: 2000000,
        });
        expect(result.byUser?.john).toEqual({
          requests: 8000,
          tokensInput: 2500000,
          tokensOutput: 1500000,
        });
      });
    });

    describe("getConfig", () => {
      it("maps config response correctly", async () => {
        mockFetch.mockResolvedValueOnce({
          ok: true,
          status: 200,
          json: () =>
            Promise.resolve({
              proxy_port: 8317,
              admin_port: 3000,
              log_level: "info",
              auto_start_proxy: true,
              model_mappings: { "gpt-4": "claude-3-opus" },
              rate_limits: {
                requests_per_minute: 60,
                tokens_per_day: 1000000,
              },
            }),
        });

        const result = await client.getConfig();

        expect(result).toEqual({
          proxyPort: 8317,
          adminPort: 3000,
          logLevel: "info",
          autoStartProxy: true,
          modelMappings: { "gpt-4": "claude-3-opus" },
          rateLimits: {
            requestsPerMinute: 60,
            tokensPerDay: 1000000,
          },
        });
      });
    });

    describe("saveConfig", () => {
      it("converts camelCase to snake_case in request body", async () => {
        mockFetch.mockResolvedValueOnce({
          ok: true,
          status: 200,
          json: () => Promise.resolve({ success: true, restart_required: true }),
        });

        await client.saveConfig({
          proxyPort: 8318,
          logLevel: "debug",
          autoStartProxy: false,
          rateLimits: {
            requestsPerMinute: 100,
            tokensPerDay: 2000000,
          },
        });

        expect(mockFetch).toHaveBeenCalledWith(
          "/api/config",
          expect.objectContaining({
            body: JSON.stringify({
              proxy_port: 8318,
              log_level: "debug",
              auto_start_proxy: false,
              rate_limits: {
                requests_per_minute: 100,
                tokens_per_day: 2000000,
              },
            }),
          })
        );
      });
    });
  });

  // ===========================================================================
  // Endpoint Tests
  // ===========================================================================

  describe("endpoints", () => {
    let client: HttpBackendClient;

    beforeEach(() => {
      client = new HttpBackendClient();
    });

    describe("startProxy", () => {
      it("returns running status with port and pid", async () => {
        mockFetch.mockResolvedValueOnce({
          ok: true,
          status: 200,
          json: () => Promise.resolve({ success: true, pid: 12345, port: 8317 }),
        });

        const result = await client.startProxy();

        expect(result).toEqual({
          running: true,
          port: 8317,
          pid: 12345,
        });
      });
    });

    describe("stopProxy", () => {
      it("returns stopped status", async () => {
        mockFetch.mockResolvedValueOnce({
          ok: true,
          status: 200,
          json: () => Promise.resolve({ success: true }),
        });

        const result = await client.stopProxy();

        expect(result).toEqual({
          running: false,
          port: 8317,
        });
      });
    });

    describe("regenerateApiKey", () => {
      it("returns new API key", async () => {
        mockFetch.mockResolvedValueOnce({
          ok: true,
          status: 200,
          json: () =>
            Promise.resolve({
              api_key: "sk-john-newkey123",
              api_key_prefix: "sk-john",
            }),
        });

        const result = await client.regenerateApiKey(1);

        expect(result).toEqual({
          apiKey: "sk-john-newkey123",
          apiKeyPrefix: "sk-john",
        });
      });
    });

    describe("resetUserUsage", () => {
      it("returns previous used tokens", async () => {
        mockFetch.mockResolvedValueOnce({
          ok: true,
          status: 200,
          json: () =>
            Promise.resolve({
              success: true,
              previous_used_tokens: 250000,
            }),
        });

        const result = await client.resetUserUsage(1);

        expect(result).toEqual({
          previousUsedTokens: 250000,
        });
      });
    });

    describe("startOAuth", () => {
      it("returns redirect URL for server mode", async () => {
        const result = await client.startOAuth("claude");

        expect(result).toEqual({
          redirectUrl: "/oauth/claude/start",
        });
      });
    });

    describe("login", () => {
      it("sends password and returns success", async () => {
        mockFetch.mockResolvedValueOnce({
          ok: true,
          status: 200,
          json: () => Promise.resolve({ success: true }),
        });

        const result = await client.login("admin-password");

        expect(mockFetch).toHaveBeenCalledWith(
          "/api/auth/login",
          expect.objectContaining({
            method: "POST",
            body: JSON.stringify({ password: "admin-password" }),
          })
        );
        expect(result).toEqual({ success: true });
      });
    });

    describe("getAuthStatus", () => {
      it("returns authentication status", async () => {
        mockFetch.mockResolvedValueOnce({
          ok: true,
          status: 200,
          json: () =>
            Promise.resolve({
              authenticated: true,
              expires_at: "2024-01-15T12:00:00Z",
            }),
        });

        const result = await client.getAuthStatus();

        expect(result).toEqual({
          authenticated: true,
          expiresAt: "2024-01-15T12:00:00Z",
        });
      });
    });

    describe("onProxyStatusChanged", () => {
      it("returns no-op unsubscribe function", async () => {
        const callback = vi.fn();
        const unlisten = await client.onProxyStatusChanged(callback);

        expect(typeof unlisten).toBe("function");
        unlisten();
        expect(callback).not.toHaveBeenCalled();
      });
    });

    describe("getRequestLogs", () => {
      it("maps request logs correctly", async () => {
        mockFetch.mockResolvedValueOnce({
          ok: true,
          status: 200,
          json: () =>
            Promise.resolve({
              logs: [
                {
                  id: 1523,
                  timestamp: "2024-01-15T10:30:00Z",
                  user_id: 1,
                  user_name: "john",
                  provider: "claude",
                  model: "claude-3-opus",
                  tokens_input: 500,
                  tokens_output: 1200,
                  duration_ms: 2500,
                  status: "success",
                },
              ],
              total: 15230,
              limit: 100,
              offset: 0,
            }),
        });

        const result = await client.getRequestLogs({ limit: 100 });

        expect(result.logs[0]).toEqual({
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
          method: undefined,
          path: undefined,
        });
      });
    });
  });
});
