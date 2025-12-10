import { render, screen, fireEvent, waitFor } from "@solidjs/testing-library";
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { UsagePage } from "../Usage";

const mockStats = {
  period: "month",
  totalRequests: 1500,
  totalTokensInput: 250000,
  totalTokensOutput: 125000,
  byProvider: {
    claude: { requests: 800, tokensInput: 150000, tokensOutput: 75000 },
    openai: { requests: 500, tokensInput: 80000, tokensOutput: 40000 },
    gemini: { requests: 200, tokensInput: 20000, tokensOutput: 10000 },
  },
  byUser: {
    alice: { requests: 900, tokensInput: 140000, tokensOutput: 70000 },
    bob: { requests: 600, tokensInput: 110000, tokensOutput: 55000 },
  },
};

const mockDailyUsage = {
  days: 30,
  data: [
    { date: "2024-12-01", requests: 50, tokensInput: 8000, tokensOutput: 4000 },
    { date: "2024-12-02", requests: 75, tokensInput: 12000, tokensOutput: 6000 },
    { date: "2024-12-03", requests: 60, tokensInput: 10000, tokensOutput: 5000 },
  ],
};

let mockIsServerMode = false;

vi.mock("../../backend", () => ({
  backendClient: {
    getUsageStats: vi.fn(() => Promise.resolve(mockStats)),
    getDailyUsage: vi.fn(() => Promise.resolve(mockDailyUsage)),
  },
  isServerMode: () => mockIsServerMode,
}));

vi.mock("../../stores/app", () => ({
  appStore: {
    setCurrentPage: vi.fn(),
  },
}));

vi.mock("chart.js", () => {
  class MockChart {
    static register() {}
    constructor() {}
    destroy() {}
    update() {}
  }
  return {
    Chart: MockChart,
    registerables: [],
  };
});

describe("UsagePage", () => {
  beforeEach(() => {
    mockIsServerMode = false;
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("renders summary cards with data", async () => {
    render(() => <UsagePage />);

    await waitFor(() => {
      expect(screen.getByText("Usage Statistics")).toBeInTheDocument();
    });

    await waitFor(() => {
      expect(screen.getByText("1.5K")).toBeInTheDocument();
      expect(screen.getByText("250K")).toBeInTheDocument();
      expect(screen.getByText("125K")).toBeInTheDocument();
      expect(screen.getByText("3")).toBeInTheDocument();
    });
  });

  it("renders period selector with all options", async () => {
    render(() => <UsagePage />);

    await waitFor(() => {
      expect(screen.getByText("Today")).toBeInTheDocument();
      expect(screen.getByText("Week")).toBeInTheDocument();
      expect(screen.getByText("Month")).toBeInTheDocument();
      expect(screen.getByText("All Time")).toBeInTheDocument();
    });
  });

  it("period selector changes trigger refetch", async () => {
    const { backendClient } = await import("../../backend");
    render(() => <UsagePage />);

    await waitFor(() => {
      expect(screen.getByText("Today")).toBeInTheDocument();
    });

    const todayButton = screen.getByText("Today");
    fireEvent.click(todayButton);

    await waitFor(() => {
      expect(backendClient.getUsageStats).toHaveBeenCalledWith("today");
    });
  });

  it("renders provider usage table", async () => {
    render(() => <UsagePage />);

    await waitFor(() => {
      expect(screen.getByText("Usage by Provider")).toBeInTheDocument();
    });

    await waitFor(() => {
      expect(screen.getByText("claude")).toBeInTheDocument();
      expect(screen.getByText("openai")).toBeInTheDocument();
      expect(screen.getByText("gemini")).toBeInTheDocument();
    });
  });

  it("hides user section in tauri mode", async () => {
    mockIsServerMode = false;
    render(() => <UsagePage />);

    await waitFor(() => {
      expect(screen.getByText("Usage by Provider")).toBeInTheDocument();
    });

    expect(screen.queryByText("Usage by User")).not.toBeInTheDocument();
  });

  it("shows user section in server mode", async () => {
    mockIsServerMode = true;
    render(() => <UsagePage />);

    await waitFor(() => {
      expect(screen.getByText("Usage by User")).toBeInTheDocument();
    });
  });

  it("renders daily usage chart section", async () => {
    render(() => <UsagePage />);

    await waitFor(() => {
      expect(screen.getByText("Daily Usage (Last 30 Days)")).toBeInTheDocument();
    });
  });

  it("refresh button triggers data refetch", async () => {
    const { backendClient } = await import("../../backend");
    render(() => <UsagePage />);

    await waitFor(() => {
      expect(screen.getByText("Refresh")).toBeInTheDocument();
    });

    const refreshButton = screen.getByText("Refresh");
    fireEvent.click(refreshButton);

    await waitFor(() => {
      expect(backendClient.getUsageStats).toHaveBeenCalled();
      expect(backendClient.getDailyUsage).toHaveBeenCalled();
    });
  });
});
