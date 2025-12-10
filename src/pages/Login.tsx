import { createSignal } from "solid-js";
import { Button } from "../components/ui";
import { appStore } from "../stores/app";
import { toastStore } from "../stores/toast";
import { backendClient } from "../backend";

export function LoginPage() {
  const { setCurrentPage, setIsAuthenticated, initialize } = appStore;
  const [password, setPassword] = createSignal("");
  const [isLoading, setIsLoading] = createSignal(false);
  const [error, setError] = createSignal("");

  const handleLogin = async (e: Event) => {
    e.preventDefault();
    setError("");
    setIsLoading(true);

    try {
      const result = await backendClient.login(password());
      if (result.success) {
        setIsAuthenticated(true);
        toastStore.success("Logged in", "Welcome to ProxyPal");
        // Re-initialize to load data after login
        await initialize();
        setCurrentPage("dashboard");
      } else {
        setError("Invalid password");
      }
    } catch (err) {
      console.error("Login failed:", err);
      setError("Login failed. Please try again.");
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div class="min-h-screen flex flex-col items-center justify-center bg-gray-50 dark:bg-gray-900">
      <div class="w-full max-w-md px-6">
        {/* Logo */}
        <div class="text-center mb-8">
          <div class="w-16 h-16 mx-auto rounded-2xl bg-gradient-to-br from-brand-500 to-brand-700 flex items-center justify-center mb-4">
            <span class="text-white text-3xl">⚡</span>
          </div>
          <h1 class="text-2xl font-bold text-gray-900 dark:text-gray-100">
            ProxyPal Server
          </h1>
          <p class="text-gray-500 dark:text-gray-400 mt-2">
            Enter admin password to continue
          </p>
        </div>

        {/* Login Form */}
        <form onSubmit={handleLogin} class="space-y-4">
          <div>
            <label
              for="password"
              class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1"
            >
              Admin Password
            </label>
            <input
              type="password"
              id="password"
              value={password()}
              onInput={(e) => setPassword(e.currentTarget.value)}
              class="w-full px-4 py-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 focus:ring-2 focus:ring-brand-500 focus:border-transparent"
              placeholder="Enter password"
              required
              autofocus
            />
          </div>

          {error() && (
            <p class="text-red-500 text-sm">{error()}</p>
          )}

          <Button
            type="submit"
            variant="primary"
            size="lg"
            class="w-full"
            disabled={isLoading()}
          >
            {isLoading() ? "Logging in..." : "Login"}
          </Button>
        </form>

        {/* Footer */}
        <p class="text-center text-xs text-gray-500 dark:text-gray-400 mt-8">
          ProxyPal Server • Shared AI Proxy
        </p>
      </div>
    </div>
  );
}
