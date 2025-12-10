import { createSignal, createResource, For, Show } from "solid-js";
import { backendClient, isServerMode } from "../backend";
import type { User, UserListResponse } from "../backend/types";
import { Button } from "../components/ui";
import { toastStore } from "../stores/toast";

function formatDate(dateStr: string | null): string {
  if (!dateStr) return "Never";
  const date = new Date(dateStr);
  return date.toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    year: "numeric",
  });
}

function formatTokens(tokens: number | null): string {
  if (tokens === null) return "Unlimited";
  if (tokens >= 1_000_000) return `${(tokens / 1_000_000).toFixed(1)}M`;
  if (tokens >= 1_000) return `${(tokens / 1_000).toFixed(1)}K`;
  return tokens.toString();
}

interface UserRowProps {
  user: User;
  onRefetch: () => void;
}

function UserRow(props: UserRowProps) {
  const [deleting, setDeleting] = createSignal(false);
  const [regenerating, setRegenerating] = createSignal(false);
  const [toggling, setToggling] = createSignal(false);
  const [showApiKeyModal, setShowApiKeyModal] = createSignal<string | null>(
    null
  );

  async function handleToggleEnabled() {
    setToggling(true);
    try {
      await backendClient.updateUser(props.user.id, {
        enabled: !props.user.enabled,
      });
      props.onRefetch();
      toastStore.success(
        `User ${props.user.enabled ? "disabled" : "enabled"}`
      );
    } catch (err) {
      toastStore.error("Failed to update user", String(err));
    } finally {
      setToggling(false);
    }
  }

  async function handleRegenerateKey() {
    if (
      !confirm(
        `Regenerate API key for ${props.user.name}? The old key will stop working immediately.`
      )
    )
      return;

    setRegenerating(true);
    try {
      const result = await backendClient.regenerateApiKey(props.user.id);
      setShowApiKeyModal(result.apiKey);
      props.onRefetch();
      toastStore.success("API key regenerated");
    } catch (err) {
      toastStore.error("Failed to regenerate key", String(err));
    } finally {
      setRegenerating(false);
    }
  }

  async function handleDelete() {
    if (
      !confirm(
        `Delete user "${props.user.name}"? This action cannot be undone.`
      )
    )
      return;

    setDeleting(true);
    try {
      await backendClient.deleteUser(props.user.id);
      props.onRefetch();
      toastStore.success(`User ${props.user.name} deleted`);
    } catch (err) {
      toastStore.error("Failed to delete user", String(err));
    } finally {
      setDeleting(false);
    }
  }

  return (
    <>
      <tr class="border-b border-gray-200 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-800/50">
        <td class="px-4 py-3 text-sm text-gray-900 dark:text-gray-100">
          {props.user.name}
        </td>
        <td class="px-4 py-3 text-sm font-mono text-gray-600 dark:text-gray-400">
          {props.user.apiKeyPrefix}...
        </td>
        <td class="px-4 py-3 text-sm text-gray-600 dark:text-gray-400">
          {formatTokens(props.user.quotaTokens)}
        </td>
        <td class="px-4 py-3 text-sm text-gray-600 dark:text-gray-400 tabular-nums">
          {formatTokens(props.user.usedTokens)}
        </td>
        <td class="px-4 py-3">
          <span
            class={`inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium ${
              props.user.enabled
                ? "bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-400"
                : "bg-red-100 dark:bg-red-900/30 text-red-700 dark:text-red-400"
            }`}
          >
            {props.user.enabled ? "Enabled" : "Disabled"}
          </span>
        </td>
        <td class="px-4 py-3 text-sm text-gray-500 dark:text-gray-400">
          {formatDate(props.user.lastUsedAt)}
        </td>
        <td class="px-4 py-3">
          <div class="flex items-center gap-2">
            <Button
              size="sm"
              variant="ghost"
              onClick={handleToggleEnabled}
              disabled={toggling()}
              title={props.user.enabled ? "Disable user" : "Enable user"}
            >
              {toggling() ? (
                <svg
                  class="w-4 h-4 animate-spin"
                  fill="none"
                  viewBox="0 0 24 24"
                >
                  <circle
                    class="opacity-25"
                    cx="12"
                    cy="12"
                    r="10"
                    stroke="currentColor"
                    stroke-width="4"
                  />
                  <path
                    class="opacity-75"
                    fill="currentColor"
                    d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"
                  />
                </svg>
              ) : props.user.enabled ? (
                <svg
                  class="w-4 h-4"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636"
                  />
                </svg>
              ) : (
                <svg
                  class="w-4 h-4"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"
                  />
                </svg>
              )}
            </Button>
            <Button
              size="sm"
              variant="ghost"
              onClick={handleRegenerateKey}
              disabled={regenerating()}
              title="Regenerate API key"
            >
              {regenerating() ? (
                <svg
                  class="w-4 h-4 animate-spin"
                  fill="none"
                  viewBox="0 0 24 24"
                >
                  <circle
                    class="opacity-25"
                    cx="12"
                    cy="12"
                    r="10"
                    stroke="currentColor"
                    stroke-width="4"
                  />
                  <path
                    class="opacity-75"
                    fill="currentColor"
                    d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"
                  />
                </svg>
              ) : (
                <svg
                  class="w-4 h-4"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
                  />
                </svg>
              )}
            </Button>
            <Button
              size="sm"
              variant="danger"
              onClick={handleDelete}
              disabled={deleting()}
              title="Delete user"
            >
              {deleting() ? (
                <svg
                  class="w-4 h-4 animate-spin"
                  fill="none"
                  viewBox="0 0 24 24"
                >
                  <circle
                    class="opacity-25"
                    cx="12"
                    cy="12"
                    r="10"
                    stroke="currentColor"
                    stroke-width="4"
                  />
                  <path
                    class="opacity-75"
                    fill="currentColor"
                    d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"
                  />
                </svg>
              ) : (
                <svg
                  class="w-4 h-4"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
                  />
                </svg>
              )}
            </Button>
          </div>
        </td>
      </tr>

      <Show when={showApiKeyModal()}>
        <ApiKeyModal
          apiKey={showApiKeyModal()!}
          onClose={() => setShowApiKeyModal(null)}
        />
      </Show>
    </>
  );
}

interface ApiKeyModalProps {
  apiKey: string;
  onClose: () => void;
}

function ApiKeyModal(props: ApiKeyModalProps) {
  const [copied, setCopied] = createSignal(false);

  async function handleCopy() {
    await navigator.clipboard.writeText(props.apiKey);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }

  return (
    <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 animate-fade-in">
      <div class="bg-white dark:bg-gray-900 rounded-2xl shadow-2xl w-full max-w-md animate-scale-in">
        <div class="p-6">
          <div class="flex items-center justify-between mb-4">
            <h2 class="text-lg font-bold text-gray-900 dark:text-gray-100">
              API Key Created
            </h2>
            <button
              onClick={props.onClose}
              class="p-1 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
            >
              <svg
                class="w-5 h-5"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M6 18L18 6M6 6l12 12"
                />
              </svg>
            </button>
          </div>

          <div class="p-3 rounded-lg bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 mb-4">
            <div class="flex items-center gap-2 text-amber-700 dark:text-amber-300">
              <svg
                class="w-4 h-4"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
                />
              </svg>
              <span class="text-sm font-medium">
                Save this key now - it won't be shown again!
              </span>
            </div>
          </div>

          <div class="space-y-3">
            <div class="p-3 rounded-lg bg-gray-100 dark:bg-gray-800 border border-gray-200 dark:border-gray-700">
              <code class="text-sm font-mono text-gray-900 dark:text-gray-100 break-all">
                {props.apiKey}
              </code>
            </div>

            <Button variant="secondary" class="w-full" onClick={handleCopy}>
              {copied() ? (
                <span class="flex items-center gap-2">
                  <svg
                    class="w-4 h-4"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M5 13l4 4L19 7"
                    />
                  </svg>
                  Copied!
                </span>
              ) : (
                <span class="flex items-center gap-2">
                  <svg
                    class="w-4 h-4"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M8 5H6a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2v-1M8 5a2 2 0 002 2h2a2 2 0 002-2M8 5a2 2 0 012-2h2a2 2 0 012 2m0 0h2a2 2 0 012 2v3m2 4H10m0 0l3-3m-3 3l3 3"
                    />
                  </svg>
                  Copy to Clipboard
                </span>
              )}
            </Button>
          </div>

          <div class="mt-6 flex justify-end">
            <Button variant="primary" onClick={props.onClose}>
              Done
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}

export function UsersPage() {
  const [users, { refetch }] = createResource<UserListResponse>(fetchUsers);
  const [newUserName, setNewUserName] = createSignal("");
  const [newUserQuota, setNewUserQuota] = createSignal<string>("");
  const [creatingUser, setCreatingUser] = createSignal(false);
  const [showApiKeyModal, setShowApiKeyModal] = createSignal<string | null>(
    null
  );

  async function fetchUsers(): Promise<UserListResponse> {
    if (!isServerMode())
      return { users: [], total: 0, page: 1, limit: 50 };
    return backendClient.listUsers();
  }

  async function handleCreateUser(e: Event) {
    e.preventDefault();
    const name = newUserName().trim();
    if (!name) {
      toastStore.error("Name is required");
      return;
    }

    setCreatingUser(true);
    try {
      const quotaStr = newUserQuota().trim();
      const quotaTokens = quotaStr ? parseInt(quotaStr, 10) : null;

      if (quotaStr && isNaN(quotaTokens!)) {
        toastStore.error("Quota must be a valid number");
        setCreatingUser(false);
        return;
      }

      const user = await backendClient.createUser({
        name,
        quotaTokens,
      });

      if (user.apiKey) {
        setShowApiKeyModal(user.apiKey);
      }

      setNewUserName("");
      setNewUserQuota("");
      refetch();
      toastStore.success(`User ${user.name} created`);
    } catch (err) {
      toastStore.error("Failed to create user", String(err));
    } finally {
      setCreatingUser(false);
    }
  }

  if (!isServerMode()) {
    return (
      <div class="min-h-screen bg-gray-100 dark:bg-gray-950 p-6">
        <div class="max-w-4xl mx-auto">
          <div class="p-8 rounded-xl bg-white dark:bg-gray-900 border border-gray-200 dark:border-gray-800 text-center">
            <svg
              class="w-16 h-16 mx-auto text-gray-400 dark:text-gray-600 mb-4"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"
              />
            </svg>
            <h2 class="text-xl font-bold text-gray-900 dark:text-gray-100 mb-2">
              Server Mode Only
            </h2>
            <p class="text-gray-600 dark:text-gray-400">
              User management is only available when running ProxyPal in server
              mode. This feature allows you to create and manage API keys for
              multiple users.
            </p>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div class="min-h-screen bg-gray-100 dark:bg-gray-950 p-6">
      <div class="max-w-6xl mx-auto space-y-6">
        <h1 class="text-2xl font-bold text-gray-900 dark:text-gray-100">
          User Management
        </h1>

        <div class="p-6 rounded-xl bg-white dark:bg-gray-900 border border-gray-200 dark:border-gray-800">
          <h2 class="text-sm font-semibold text-gray-600 dark:text-gray-400 uppercase tracking-wider mb-4">
            Create New User
          </h2>
          <form onSubmit={handleCreateUser} class="flex items-end gap-4">
            <div class="flex-1">
              <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                Name
              </label>
              <input
                type="text"
                value={newUserName()}
                onInput={(e) => setNewUserName(e.currentTarget.value)}
                placeholder="e.g., John Doe"
                class="w-full px-3 py-2 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-lg text-sm focus:ring-2 focus:ring-brand-500 focus:border-transparent"
              />
            </div>
            <div class="w-48">
              <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                Token Quota
                <span class="text-gray-400 font-normal"> (optional)</span>
              </label>
              <input
                type="text"
                value={newUserQuota()}
                onInput={(e) => setNewUserQuota(e.currentTarget.value)}
                placeholder="Unlimited"
                class="w-full px-3 py-2 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-lg text-sm focus:ring-2 focus:ring-brand-500 focus:border-transparent"
              />
            </div>
            <Button
              type="submit"
              variant="primary"
              disabled={creatingUser() || !newUserName().trim()}
              loading={creatingUser()}
            >
              Create User
            </Button>
          </form>
        </div>

        <div class="rounded-xl bg-white dark:bg-gray-900 border border-gray-200 dark:border-gray-800 overflow-hidden">
          <Show
            when={!users.loading}
            fallback={
              <div class="p-8 text-center text-gray-500 dark:text-gray-400">
                <svg
                  class="w-8 h-8 animate-spin mx-auto mb-2"
                  fill="none"
                  viewBox="0 0 24 24"
                >
                  <circle
                    class="opacity-25"
                    cx="12"
                    cy="12"
                    r="10"
                    stroke="currentColor"
                    stroke-width="4"
                  />
                  <path
                    class="opacity-75"
                    fill="currentColor"
                    d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"
                  />
                </svg>
                Loading users...
              </div>
            }
          >
            <Show
              when={users()?.users.length}
              fallback={
                <div class="p-8 text-center text-gray-500 dark:text-gray-400">
                  <svg
                    class="w-12 h-12 mx-auto mb-2 text-gray-300 dark:text-gray-600"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0z"
                    />
                  </svg>
                  No users yet. Create one above to get started.
                </div>
              }
            >
              <div class="overflow-x-auto">
                <table class="w-full">
                  <thead class="bg-gray-50 dark:bg-gray-800/50 border-b border-gray-200 dark:border-gray-700">
                    <tr>
                      <th class="px-4 py-3 text-left text-xs font-semibold text-gray-600 dark:text-gray-400 uppercase tracking-wider">
                        Name
                      </th>
                      <th class="px-4 py-3 text-left text-xs font-semibold text-gray-600 dark:text-gray-400 uppercase tracking-wider">
                        API Key Prefix
                      </th>
                      <th class="px-4 py-3 text-left text-xs font-semibold text-gray-600 dark:text-gray-400 uppercase tracking-wider">
                        Quota
                      </th>
                      <th class="px-4 py-3 text-left text-xs font-semibold text-gray-600 dark:text-gray-400 uppercase tracking-wider">
                        Used
                      </th>
                      <th class="px-4 py-3 text-left text-xs font-semibold text-gray-600 dark:text-gray-400 uppercase tracking-wider">
                        Status
                      </th>
                      <th class="px-4 py-3 text-left text-xs font-semibold text-gray-600 dark:text-gray-400 uppercase tracking-wider">
                        Last Used
                      </th>
                      <th class="px-4 py-3 text-left text-xs font-semibold text-gray-600 dark:text-gray-400 uppercase tracking-wider">
                        Actions
                      </th>
                    </tr>
                  </thead>
                  <tbody>
                    <For each={users()?.users}>
                      {(user) => <UserRow user={user} onRefetch={refetch} />}
                    </For>
                  </tbody>
                </table>
              </div>
            </Show>
          </Show>
        </div>

        <Show when={showApiKeyModal()}>
          <ApiKeyModal
            apiKey={showApiKeyModal()!}
            onClose={() => setShowApiKeyModal(null)}
          />
        </Show>
      </div>
    </div>
  );
}
