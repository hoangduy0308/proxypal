import { render } from "@solidjs/testing-library";
import { describe, it, expect, vi } from "vitest";
import App from "../App";

vi.mock("../stores/app", () => ({
  appStore: {
    currentPage: () => "welcome",
    isInitialized: () => true,
    initialize: vi.fn(),
  },
}));

vi.mock("../pages", () => ({
  WelcomePage: () => <div data-testid="welcome-page">Welcome</div>,
  DashboardPage: () => <div>Dashboard</div>,
  SettingsPage: () => <div>Settings</div>,
  ApiKeysPage: () => <div>ApiKeys</div>,
  AuthFilesPage: () => <div>AuthFiles</div>,
  LogViewerPage: () => <div>LogViewer</div>,
  AnalyticsPage: () => <div>Analytics</div>,
}));

vi.mock("../components/ui", () => ({
  ToastContainer: () => null,
}));

vi.mock("../components/CommandPalette", () => ({
  CommandPalette: () => null,
}));

describe("App", () => {
  it("renders without crashing", () => {
    const { getByTestId } = render(() => <App />);
    expect(getByTestId("welcome-page")).toBeInTheDocument();
  });
});
