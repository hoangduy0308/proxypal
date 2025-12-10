import { createSignal, createRoot, onCleanup } from "solid-js";
import { backendClient, isTauri } from "../backend";
import type { ProxyStatus as BackendProxyStatus } from "../backend";
import type {
  ProxyStatus,
  AuthStatus,
  AppConfig,
  OAuthCallback,
} from "../lib/tauri";
import {
  getAuthStatus,
  refreshAuthStatus,
  completeOAuth,
  onProxyStatusChanged,
  onAuthStatusChanged,
  onOAuthCallback,
  onTrayToggleProxy,
  showSystemNotification,
} from "../lib/tauri";

function createAppStore() {
  // Proxy state
  const [proxyStatus, setProxyStatus] = createSignal<ProxyStatus>({
    running: false,
    port: 8317,
    endpoint: "http://localhost:8317/v1",
  });

  // Auth state
  const [authStatus, setAuthStatus] = createSignal<AuthStatus>({
    claude: 0,
    openai: 0,
    gemini: 0,
    qwen: 0,
    iflow: 0,
    vertex: 0,
    antigravity: 0,
  });

  // Config
  const [config, setConfig] = createSignal<AppConfig>({
    port: 8317,
    autoStart: true,
    launchAtLogin: false,
    debug: false,
    proxyUrl: "",
    requestRetry: 0,
    quotaSwitchProject: false,
    quotaSwitchPreviewModel: false,
    usageStatsEnabled: true,
    requestLogging: false,
    loggingToFile: false,
    ampApiKey: "",
    ampModelMappings: [],
    ampOpenaiProvider: undefined,
    ampOpenaiProviders: [],
    ampRoutingMode: "mappings",
    copilot: {
      enabled: false,
      port: 4141,
      accountType: "individual",
      githubToken: "",
      rateLimit: undefined,
      rateLimitWait: false,
    },
  });

  // UI state
  const [currentPage, setCurrentPage] = createSignal<
    | "welcome"
    | "dashboard"
    | "settings"
    | "api-keys"
    | "auth-files"
    | "logs"
    | "analytics"
    | "users"
    | "usage"
  >("welcome");
  const [isLoading, setIsLoading] = createSignal(false);
  const [isInitialized, setIsInitialized] = createSignal(false);

  // Proxy uptime tracking
  const [proxyStartTime, setProxyStartTime] = createSignal<number | null>(null);

  // Convert backend ProxyStatus to Tauri ProxyStatus format
  const mapBackendProxyStatus = (status: BackendProxyStatus): ProxyStatus => ({
    running: status.running,
    port: status.port,
    endpoint: status.endpoint ?? `http://localhost:${status.port}/v1`,
  });

  // Helper to update proxy status and track uptime
  const updateProxyStatus = (status: ProxyStatus, showNotification = false) => {
    const wasRunning = proxyStatus().running;
    setProxyStatus(status);

    // Track start time when proxy starts
    if (status.running && !wasRunning) {
      setProxyStartTime(Date.now());
      if (showNotification && isTauri()) {
        showSystemNotification("ProxyPal", "Proxy server is now running");
      }
    } else if (!status.running && wasRunning) {
      setProxyStartTime(null);
      if (showNotification && isTauri()) {
        showSystemNotification("ProxyPal", "Proxy server has stopped");
      }
    }
  };

  // Polling interval for HTTP mode
  let pollingInterval: ReturnType<typeof setInterval> | null = null;

  // Auth state for HTTP mode
  const [isAuthenticated, setIsAuthenticated] = createSignal(false);

  // Initialize from backend
  const initialize = async () => {
    try {
      setIsLoading(true);

      // In HTTP mode, check auth first before loading data
      if (!isTauri()) {
        try {
          const authResult = await backendClient.getAuthStatus();
          if (!authResult.authenticated) {
            // Not logged in - show login page but mark as initialized
            setCurrentPage("welcome");
            setIsInitialized(true);
            setIsLoading(false);
            return;
          }
          setIsAuthenticated(true);
        } catch {
          // Auth check failed - show login page
          setCurrentPage("welcome");
          setIsInitialized(true);
          setIsLoading(false);
          return;
        }
      }

      // Load initial state from backend via adapter (works for both modes)
      const [proxyState, configState] = await Promise.all([
        backendClient.getProxyStatus(),
        backendClient.getConfig(),
      ]);

      updateProxyStatus(mapBackendProxyStatus(proxyState));
      // Map backend config to Tauri config format
      setConfig((prev) => ({
        ...prev,
        port: configState.proxyPort,
        autoStart: configState.autoStartProxy ?? prev.autoStart,
      }));

      // Auth handling differs between modes
      if (isTauri()) {
        // Tauri mode: use Tauri-specific auth functions
        try {
          const authState = await refreshAuthStatus();
          setAuthStatus(authState);

          const hasAnyAuth =
            authState.claude ||
            authState.openai ||
            authState.gemini ||
            authState.qwen ||
            authState.iflow ||
            authState.vertex ||
            authState.antigravity;
          if (hasAnyAuth) {
            setCurrentPage("dashboard");
          }
        } catch {
          const authState = await getAuthStatus();
          setAuthStatus(authState);

          const hasAnyAuth =
            authState.claude ||
            authState.openai ||
            authState.gemini ||
            authState.qwen ||
            authState.iflow ||
            authState.vertex ||
            authState.antigravity;
          if (hasAnyAuth) {
            setCurrentPage("dashboard");
          }
        }

        // Setup event listeners only in Tauri mode
        const unlistenProxy = await onProxyStatusChanged((status) => {
          updateProxyStatus(status);
        });

        const unlistenAuth = await onAuthStatusChanged((status) => {
          setAuthStatus(status);
        });

        const unlistenOAuth = await onOAuthCallback(
          async (data: OAuthCallback) => {
            try {
              const newAuthStatus = await completeOAuth(data.provider, data.code);
              setAuthStatus(newAuthStatus);
              setCurrentPage("dashboard");
            } catch (error) {
              console.error("Failed to complete OAuth:", error);
            }
          },
        );

        const unlistenTray = await onTrayToggleProxy(async (shouldStart) => {
          try {
            if (shouldStart) {
              const status = await backendClient.startProxy();
              updateProxyStatus(mapBackendProxyStatus(status), true);
            } else {
              const status = await backendClient.stopProxy();
              updateProxyStatus(mapBackendProxyStatus(status), true);
            }
          } catch (error) {
            console.error("Failed to toggle proxy:", error);
          }
        });

        // Cleanup on unmount
        onCleanup(() => {
          unlistenProxy();
          unlistenAuth();
          unlistenOAuth();
          unlistenTray();
        });
      } else {
        // HTTP mode: no Tauri event listeners, use polling for status updates
        setCurrentPage("dashboard");

        pollingInterval = setInterval(async () => {
          try {
            const status = await backendClient.getProxyStatus();
            updateProxyStatus(mapBackendProxyStatus(status));
          } catch (error) {
            console.error("Failed to poll proxy status:", error);
          }
        }, 30000);

        onCleanup(() => {
          if (pollingInterval) {
            clearInterval(pollingInterval);
            pollingInterval = null;
          }
        });
      }

      // Auto-start proxy if configured (Tauri mode only)
      if (isTauri() && configState.autoStartProxy) {
        try {
          const status = await backendClient.startProxy();
          updateProxyStatus(mapBackendProxyStatus(status));
        } catch (error) {
          console.error("Failed to auto-start proxy:", error);
        }
      }

      setIsInitialized(true);
    } catch (error) {
      console.error("Failed to initialize app:", error);
    } finally {
      setIsLoading(false);
    }
  };

  return {
    // Proxy
    proxyStatus,
    setProxyStatus: updateProxyStatus,
    proxyStartTime,

    // Auth
    authStatus,
    setAuthStatus,
    isAuthenticated,
    setIsAuthenticated,

    // Config
    config,
    setConfig,

    // UI
    currentPage,
    setCurrentPage,
    isLoading,
    setIsLoading,
    isInitialized,

    // Actions
    initialize,
  };
}

export const appStore = createRoot(createAppStore);
