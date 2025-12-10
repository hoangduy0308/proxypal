import {
  createSignal,
  createResource,
  createEffect,
  onMount,
  onCleanup,
  Show,
  For,
} from "solid-js";
import { Chart, registerables } from "chart.js";
import { backendClient, isServerMode } from "../backend";
import type { UsageStats, DailyUsage, UsageBreakdown } from "../backend/types";
import { appStore } from "../stores/app";

Chart.register(...registerables);

type Period = "today" | "week" | "month" | "all";

function formatNumber(num: number): string {
  if (num >= 1_000_000) {
    return (num / 1_000_000).toFixed(1).replace(/\.0$/, "") + "M";
  }
  if (num >= 1_000) {
    return (num / 1_000).toFixed(1).replace(/\.0$/, "") + "K";
  }
  return num.toLocaleString();
}

function formatTokens(num: number): string {
  if (num >= 1_000_000) {
    return (num / 1_000_000).toFixed(2) + "M";
  }
  if (num >= 1_000) {
    return (num / 1_000).toFixed(1) + "K";
  }
  return num.toLocaleString();
}

function StatCard(props: {
  title: string;
  value: number | string;
  loading?: boolean;
  icon: "requests" | "tokens" | "tokensIn" | "tokensOut" | "providers";
  color: "blue" | "purple" | "green" | "orange";
  subtext?: string;
}) {
  const colors = {
    blue: "bg-blue-50 dark:bg-blue-900/20 border-blue-100 dark:border-blue-800/50 text-blue-700 dark:text-blue-300",
    purple:
      "bg-purple-50 dark:bg-purple-900/20 border-purple-100 dark:border-purple-800/50 text-purple-700 dark:text-purple-300",
    green:
      "bg-green-50 dark:bg-green-900/20 border-green-100 dark:border-green-800/50 text-green-700 dark:text-green-300",
    orange:
      "bg-orange-50 dark:bg-orange-900/20 border-orange-100 dark:border-orange-800/50 text-orange-700 dark:text-orange-300",
  };

  const icons = {
    requests: (
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
          d="M13 10V3L4 14h7v7l9-11h-7z"
        />
      </svg>
    ),
    tokens: (
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
          d="M7 21a4 4 0 01-4-4V5a2 2 0 012-2h4a2 2 0 012 2v12a4 4 0 01-4 4zm0 0h12a2 2 0 002-2v-4a2 2 0 00-2-2h-2.343M11 7.343l1.657-1.657a2 2 0 012.828 0l2.829 2.829a2 2 0 010 2.828l-8.486 8.485M7 17h.01"
        />
      </svg>
    ),
    tokensIn: (
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
          d="M19 14l-7 7m0 0l-7-7m7 7V3"
        />
      </svg>
    ),
    tokensOut: (
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
          d="M5 10l7-7m0 0l7 7m-7-7v18"
        />
      </svg>
    ),
    providers: (
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
          d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"
        />
      </svg>
    ),
  };

  return (
    <div class={`p-4 rounded-xl border ${colors[props.color]}`}>
      <Show
        when={!props.loading}
        fallback={
          <div class="animate-pulse">
            <div class="h-4 w-24 bg-current opacity-20 rounded mb-2" />
            <div class="h-8 w-16 bg-current opacity-20 rounded" />
          </div>
        }
      >
        <div class="flex items-center gap-2 mb-2 opacity-80">
          {icons[props.icon]}
          <span class="text-xs font-medium uppercase tracking-wider">
            {props.title}
          </span>
        </div>
        <p class="text-2xl font-bold tabular-nums">
          {typeof props.value === "number" ? formatNumber(props.value) : props.value}
        </p>
        <Show when={props.subtext}>
          <p class="text-xs opacity-70 mt-1">{props.subtext}</p>
        </Show>
      </Show>
    </div>
  );
}

function PeriodSelector(props: {
  value: Period;
  onChange: (period: Period) => void;
}) {
  const periods: { label: string; value: Period }[] = [
    { label: "Today", value: "today" },
    { label: "Week", value: "week" },
    { label: "Month", value: "month" },
    { label: "All Time", value: "all" },
  ];

  return (
    <div class="flex items-center bg-gray-100 dark:bg-gray-700 rounded-lg p-1">
      <For each={periods}>
        {(p) => (
          <button
            onClick={() => props.onChange(p.value)}
            class={`px-3 py-1.5 text-sm font-medium rounded-md transition-colors ${
              props.value === p.value
                ? "bg-white dark:bg-gray-600 text-gray-900 dark:text-gray-100 shadow-sm"
                : "text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200"
            }`}
          >
            {p.label}
          </button>
        )}
      </For>
    </div>
  );
}

function ProviderUsageTable(props: {
  data?: Record<string, UsageBreakdown>;
  loading?: boolean;
}) {
  const entries = () =>
    Object.entries(props.data ?? {}).sort(
      (a, b) => b[1].requests - a[1].requests
    );

  const getProviderColor = (provider: string) => {
    switch (provider.toLowerCase()) {
      case "claude":
        return "bg-orange-100 dark:bg-orange-900/30 text-orange-700 dark:text-orange-400";
      case "openai":
        return "bg-emerald-100 dark:bg-emerald-900/30 text-emerald-700 dark:text-emerald-400";
      case "gemini":
        return "bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-400";
      case "qwen":
        return "bg-purple-100 dark:bg-purple-900/30 text-purple-700 dark:text-purple-400";
      case "vertex":
        return "bg-cyan-100 dark:bg-cyan-900/30 text-cyan-700 dark:text-cyan-400";
      default:
        return "bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400";
    }
  };

  return (
    <Show
      when={!props.loading}
      fallback={
        <div class="animate-pulse space-y-2">
          <div class="h-8 bg-gray-200 dark:bg-gray-700 rounded" />
          <div class="h-8 bg-gray-200 dark:bg-gray-700 rounded" />
          <div class="h-8 bg-gray-200 dark:bg-gray-700 rounded" />
        </div>
      }
    >
      <Show
        when={entries().length > 0}
        fallback={
          <p class="text-sm text-gray-500 dark:text-gray-400 text-center py-4">
            No provider usage data
          </p>
        }
      >
        <div class="overflow-x-auto">
          <table class="w-full">
            <thead>
              <tr class="text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                <th class="pb-3">Provider</th>
                <th class="pb-3 text-right">Requests</th>
                <th class="pb-3 text-right">Input Tokens</th>
                <th class="pb-3 text-right">Output Tokens</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-gray-100 dark:divide-gray-700">
              <For each={entries()}>
                {([provider, usage]) => (
                  <tr class="hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors">
                    <td class="py-3">
                      <span
                        class={`px-2 py-1 rounded text-sm font-medium capitalize ${getProviderColor(provider)}`}
                      >
                        {provider}
                      </span>
                    </td>
                    <td class="py-3 text-right tabular-nums text-gray-700 dark:text-gray-300">
                      {formatNumber(usage.requests)}
                    </td>
                    <td class="py-3 text-right tabular-nums text-gray-700 dark:text-gray-300">
                      {formatTokens(usage.tokensInput)}
                    </td>
                    <td class="py-3 text-right tabular-nums text-gray-700 dark:text-gray-300">
                      {formatTokens(usage.tokensOutput)}
                    </td>
                  </tr>
                )}
              </For>
            </tbody>
          </table>
        </div>
      </Show>
    </Show>
  );
}

function UserUsageTable(props: {
  data?: Record<string, UsageBreakdown>;
  loading?: boolean;
}) {
  const entries = () =>
    Object.entries(props.data ?? {}).sort(
      (a, b) => b[1].requests - a[1].requests
    );

  return (
    <Show
      when={!props.loading}
      fallback={
        <div class="animate-pulse space-y-2">
          <div class="h-8 bg-gray-200 dark:bg-gray-700 rounded" />
          <div class="h-8 bg-gray-200 dark:bg-gray-700 rounded" />
        </div>
      }
    >
      <Show
        when={entries().length > 0}
        fallback={
          <p class="text-sm text-gray-500 dark:text-gray-400 text-center py-4">
            No user usage data
          </p>
        }
      >
        <div class="overflow-x-auto">
          <table class="w-full">
            <thead>
              <tr class="text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                <th class="pb-3">User</th>
                <th class="pb-3 text-right">Requests</th>
                <th class="pb-3 text-right">Input Tokens</th>
                <th class="pb-3 text-right">Output Tokens</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-gray-100 dark:divide-gray-700">
              <For each={entries()}>
                {([user, usage]) => (
                  <tr class="hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors">
                    <td class="py-3">
                      <span class="font-medium text-gray-900 dark:text-gray-100">
                        {user}
                      </span>
                    </td>
                    <td class="py-3 text-right tabular-nums text-gray-700 dark:text-gray-300">
                      {formatNumber(usage.requests)}
                    </td>
                    <td class="py-3 text-right tabular-nums text-gray-700 dark:text-gray-300">
                      {formatTokens(usage.tokensInput)}
                    </td>
                    <td class="py-3 text-right tabular-nums text-gray-700 dark:text-gray-300">
                      {formatTokens(usage.tokensOutput)}
                    </td>
                  </tr>
                )}
              </For>
            </tbody>
          </table>
        </div>
      </Show>
    </Show>
  );
}

function DailyUsageChart(props: {
  data: DailyUsage["data"];
  loading?: boolean;
}) {
  let canvasRef: HTMLCanvasElement | undefined;
  let chartInstance: Chart | null = null;

  const isDark = () => document.documentElement.classList.contains("dark");

  const createChart = () => {
    if (!canvasRef || props.data.length === 0) return;

    if (chartInstance) {
      chartInstance.destroy();
    }

    const textColor = isDark() ? "#9CA3AF" : "#6B7280";
    const gridColor = isDark()
      ? "rgba(75, 85, 99, 0.3)"
      : "rgba(209, 213, 219, 0.5)";

    const labels = props.data.map((d) => {
      const date = new Date(d.date);
      return date.toLocaleDateString("en-US", {
        month: "short",
        day: "numeric",
      });
    });

    chartInstance = new Chart(canvasRef, {
      type: "bar",
      data: {
        labels,
        datasets: [
          {
            label: "Requests",
            data: props.data.map((d) => d.requests),
            backgroundColor: "rgba(59, 130, 246, 0.7)",
            borderColor: "rgb(59, 130, 246)",
            borderWidth: 1,
            borderRadius: 4,
          },
        ],
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        plugins: {
          legend: {
            display: false,
          },
          tooltip: {
            mode: "index",
            intersect: false,
            backgroundColor: isDark() ? "#1F2937" : "#FFFFFF",
            titleColor: isDark() ? "#F3F4F6" : "#111827",
            bodyColor: isDark() ? "#D1D5DB" : "#4B5563",
            borderColor: isDark() ? "#374151" : "#E5E7EB",
            borderWidth: 1,
            padding: 12,
            cornerRadius: 8,
            callbacks: {
              afterBody: (context) => {
                const idx = context[0].dataIndex;
                const d = props.data[idx];
                return [
                  `Input: ${formatTokens(d.tokensInput)}`,
                  `Output: ${formatTokens(d.tokensOutput)}`,
                ];
              },
            },
          },
        },
        scales: {
          x: {
            grid: {
              color: gridColor,
            },
            ticks: {
              color: textColor,
              maxRotation: 45,
              minRotation: 0,
            },
          },
          y: {
            beginAtZero: true,
            grid: {
              color: gridColor,
            },
            ticks: {
              color: textColor,
            },
          },
        },
      },
    });
  };

  onMount(() => {
    createChart();
  });

  createEffect(() => {
    const data = props.data;
    if (data && canvasRef) {
      createChart();
    }
  });

  onCleanup(() => {
    if (chartInstance) {
      chartInstance.destroy();
    }
  });

  return (
    <Show
      when={!props.loading}
      fallback={
        <div class="h-64 bg-gray-200 dark:bg-gray-700 rounded-lg animate-pulse" />
      }
    >
      <Show
        when={props.data.length > 0}
        fallback={
          <div class="h-64 flex items-center justify-center text-gray-500 dark:text-gray-400">
            No daily usage data available
          </div>
        }
      >
        <div class="h-64">
          <canvas ref={canvasRef} class="w-full h-full" />
        </div>
      </Show>
    </Show>
  );
}

export function UsagePage() {
  const { setCurrentPage } = appStore;
  const [period, setPeriod] = createSignal<Period>("month");
  const [refreshing, setRefreshing] = createSignal(false);

  const [stats, { refetch: refetchStats }] = createResource<UsageStats | null, Period>(
    period,
    async (p) => {
      try {
        return await backendClient.getUsageStats(p);
      } catch (err) {
        console.error("Failed to fetch usage stats:", err);
        return null;
      }
    }
  );

  const [dailyUsage, { refetch: refetchDaily }] = createResource<DailyUsage | null>(
    async () => {
      try {
        return await backendClient.getDailyUsage({ days: 30 });
      } catch (err) {
        console.error("Failed to fetch daily usage:", err);
        return null;
      }
    }
  );

  const handleRefresh = async () => {
    setRefreshing(true);
    await Promise.all([refetchStats(), refetchDaily()]);
    setRefreshing(false);
  };

  const activeProviders = () => Object.keys(stats()?.byProvider ?? {}).length;

  return (
    <div class="min-h-screen bg-gray-50 dark:bg-gray-900 p-4 sm:p-6">
      <div class="max-w-6xl mx-auto space-y-6">
        {/* Header */}
        <div class="flex items-center justify-between flex-wrap gap-4">
          <div class="flex items-center gap-3">
            <button
              onClick={() => setCurrentPage("dashboard")}
              class="p-2 rounded-lg hover:bg-gray-200 dark:hover:bg-gray-800 transition-colors"
            >
              <svg
                class="w-5 h-5 text-gray-600 dark:text-gray-400"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M15 19l-7-7 7-7"
                />
              </svg>
            </button>
            <div>
              <h1 class="text-xl sm:text-2xl font-bold text-gray-900 dark:text-gray-100">
                Usage Statistics
              </h1>
              <p class="text-sm text-gray-500 dark:text-gray-400">
                Track requests and token consumption
              </p>
            </div>
          </div>

          <div class="flex items-center gap-3">
            <PeriodSelector value={period()} onChange={setPeriod} />
            <button
              onClick={handleRefresh}
              disabled={refreshing()}
              class="flex items-center gap-2 px-3 py-1.5 text-sm font-medium text-gray-700 dark:text-gray-300 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 disabled:opacity-50 transition-colors"
            >
              <svg
                class={`w-4 h-4 ${refreshing() ? "animate-spin" : ""}`}
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
              Refresh
            </button>
          </div>
        </div>

        {/* Summary Cards */}
        <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
          <StatCard
            title="Total Requests"
            value={stats()?.totalRequests ?? 0}
            loading={stats.loading}
            icon="requests"
            color="blue"
          />
          <StatCard
            title="Input Tokens"
            value={stats()?.totalTokensInput ?? 0}
            loading={stats.loading}
            icon="tokensIn"
            color="purple"
          />
          <StatCard
            title="Output Tokens"
            value={stats()?.totalTokensOutput ?? 0}
            loading={stats.loading}
            icon="tokensOut"
            color="green"
          />
          <StatCard
            title="Active Providers"
            value={activeProviders()}
            loading={stats.loading}
            icon="providers"
            color="orange"
          />
        </div>

        {/* Usage by Provider */}
        <section class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-4 sm:p-6">
          <h2 class="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-4">
            Usage by Provider
          </h2>
          <ProviderUsageTable
            data={stats()?.byProvider}
            loading={stats.loading}
          />
        </section>

        {/* Usage by User (server mode only) */}
        <Show when={isServerMode()}>
          <section class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-4 sm:p-6">
            <h2 class="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-4">
              Usage by User
            </h2>
            <UserUsageTable data={stats()?.byUser} loading={stats.loading} />
          </section>
        </Show>

        {/* Daily Usage Chart */}
        <section class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-4 sm:p-6">
          <h2 class="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-4">
            Daily Usage (Last 30 Days)
          </h2>
          <DailyUsageChart
            data={dailyUsage()?.data ?? []}
            loading={dailyUsage.loading}
          />
        </section>
      </div>
    </div>
  );
}
