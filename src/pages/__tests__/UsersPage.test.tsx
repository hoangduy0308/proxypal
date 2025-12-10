/**
 * Tests for UsersPage component
 */

import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { render, screen, fireEvent, waitFor, cleanup } from "@solidjs/testing-library";

vi.mock("../../backend", () => ({
  isServerMode: vi.fn(),
  backendClient: {
    listUsers: vi.fn(),
    createUser: vi.fn(),
    updateUser: vi.fn(),
    deleteUser: vi.fn(),
    regenerateApiKey: vi.fn(),
  },
}));

vi.mock("../../stores/toast", () => ({
  toastStore: {
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
    warning: vi.fn(),
  },
}));

import { UsersPage } from "../Users";
import { isServerMode, backendClient } from "../../backend";
import { toastStore } from "../../stores/toast";
import type { User, UserListResponse, CreateUserResponse } from "../../backend/types";

const mockIsServerMode = isServerMode as unknown as ReturnType<typeof vi.fn>;
const mockBackendClient = backendClient as unknown as {
  listUsers: ReturnType<typeof vi.fn>;
  createUser: ReturnType<typeof vi.fn>;
  updateUser: ReturnType<typeof vi.fn>;
  deleteUser: ReturnType<typeof vi.fn>;
  regenerateApiKey: ReturnType<typeof vi.fn>;
};
const mockToastStore = toastStore as unknown as {
  success: ReturnType<typeof vi.fn>;
  error: ReturnType<typeof vi.fn>;
};

const mockUsers: User[] = [
  {
    id: 1,
    name: "John Doe",
    apiKeyPrefix: "sk-john",
    quotaTokens: 100000,
    usedTokens: 5000,
    enabled: true,
    createdAt: "2024-01-01T00:00:00Z",
    lastUsedAt: "2024-01-15T12:00:00Z",
  },
  {
    id: 2,
    name: "Jane Smith",
    apiKeyPrefix: "sk-jane",
    quotaTokens: null,
    usedTokens: 25000,
    enabled: false,
    createdAt: "2024-01-02T00:00:00Z",
    lastUsedAt: null,
  },
];

const mockUserListResponse: UserListResponse = {
  users: mockUsers,
  total: 2,
  page: 1,
  limit: 50,
};

describe("UsersPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockIsServerMode.mockReturnValue(true);
    mockBackendClient.listUsers.mockResolvedValue(mockUserListResponse);
  });

  afterEach(() => {
    cleanup();
  });

  describe("Server mode check", () => {
    it("shows server-mode-only message when not in server mode", async () => {
      mockIsServerMode.mockReturnValue(false);

      render(() => <UsersPage />);

      expect(screen.getByText("Server Mode Only")).toBeInTheDocument();
      expect(
        screen.getByText(/User management is only available/)
      ).toBeInTheDocument();
    });

    it("renders user management UI when in server mode", async () => {
      mockIsServerMode.mockReturnValue(true);

      render(() => <UsersPage />);

      await waitFor(() => {
        expect(screen.getByText("User Management")).toBeInTheDocument();
      });
    });
  });

  describe("User list rendering", () => {
    it("renders user list from backend", async () => {
      render(() => <UsersPage />);

      await waitFor(() => {
        expect(screen.getByText("John Doe")).toBeInTheDocument();
        expect(screen.getByText("Jane Smith")).toBeInTheDocument();
      });
    });

    it("displays API key prefix with ellipsis", async () => {
      render(() => <UsersPage />);

      await waitFor(() => {
        expect(screen.getByText("sk-john...")).toBeInTheDocument();
        expect(screen.getByText("sk-jane...")).toBeInTheDocument();
      });
    });

    it("displays formatted quota tokens", async () => {
      render(() => <UsersPage />);

      await waitFor(() => {
        expect(screen.getByText("100.0K")).toBeInTheDocument();
        expect(screen.getByText("Unlimited")).toBeInTheDocument();
      });
    });

    it("displays enabled/disabled status", async () => {
      render(() => <UsersPage />);

      await waitFor(() => {
        expect(screen.getByText("Enabled")).toBeInTheDocument();
        expect(screen.getByText("Disabled")).toBeInTheDocument();
      });
    });

    it("shows loading state while fetching users", async () => {
      mockBackendClient.listUsers.mockImplementation(
        () => new Promise(() => {})
      );

      render(() => <UsersPage />);

      expect(screen.getByText("Loading users...")).toBeInTheDocument();
    });

    it("shows empty state when no users exist", async () => {
      mockBackendClient.listUsers.mockResolvedValue({
        users: [],
        total: 0,
        page: 1,
        limit: 50,
      });

      render(() => <UsersPage />);

      await waitFor(() => {
        expect(
          screen.getByText(/No users yet. Create one above/)
        ).toBeInTheDocument();
      });
    });
  });

  describe("Create user flow", () => {
    it("creates user with name and optional quota", async () => {
      const newUser: CreateUserResponse = {
        id: 3,
        name: "New User",
        apiKeyPrefix: "sk-new",
        quotaTokens: 50000,
        usedTokens: 0,
        enabled: true,
        createdAt: "2024-01-20T00:00:00Z",
        lastUsedAt: null,
        apiKey: "sk-new-full-api-key-12345",
      };

      mockBackendClient.createUser.mockResolvedValue(newUser);

      render(() => <UsersPage />);

      await waitFor(() => {
        expect(screen.getByText("Create New User")).toBeInTheDocument();
      });

      const nameInput = screen.getByPlaceholderText("e.g., John Doe");
      const quotaInput = screen.getByPlaceholderText("Unlimited");
      const submitButton = screen.getByText("Create User");

      fireEvent.input(nameInput, { target: { value: "New User" } });
      fireEvent.input(quotaInput, { target: { value: "50000" } });
      fireEvent.click(submitButton);

      await waitFor(() => {
        expect(mockBackendClient.createUser).toHaveBeenCalledWith({
          name: "New User",
          quotaTokens: 50000,
        });
      });

      await waitFor(() => {
        expect(mockToastStore.success).toHaveBeenCalledWith("User New User created");
      });
    });

    it("shows API key modal after creation", async () => {
      const newUser: CreateUserResponse = {
        id: 3,
        name: "New User",
        apiKeyPrefix: "sk-new",
        quotaTokens: null,
        usedTokens: 0,
        enabled: true,
        createdAt: "2024-01-20T00:00:00Z",
        lastUsedAt: null,
        apiKey: "sk-new-full-api-key-12345",
      };

      mockBackendClient.createUser.mockResolvedValue(newUser);

      render(() => <UsersPage />);

      await waitFor(() => {
        expect(screen.getByText("Create New User")).toBeInTheDocument();
      });

      const nameInput = screen.getByPlaceholderText("e.g., John Doe");
      fireEvent.input(nameInput, { target: { value: "New User" } });
      fireEvent.click(screen.getByText("Create User"));

      await waitFor(() => {
        expect(screen.getByText("API Key Created")).toBeInTheDocument();
        expect(
          screen.getByText("sk-new-full-api-key-12345")
        ).toBeInTheDocument();
      });
    });

    it("shows error when name is empty", async () => {
      render(() => <UsersPage />);

      await waitFor(() => {
        expect(screen.getByText("Create New User")).toBeInTheDocument();
      });

      const submitButton = screen.getByText("Create User");
      expect(submitButton).toBeDisabled();
    });

    it("handles create user error", async () => {
      mockBackendClient.createUser.mockRejectedValue(new Error("Server error"));

      render(() => <UsersPage />);

      await waitFor(() => {
        expect(screen.getByText("Create New User")).toBeInTheDocument();
      });

      const nameInput = screen.getByPlaceholderText("e.g., John Doe");
      fireEvent.input(nameInput, { target: { value: "New User" } });
      fireEvent.click(screen.getByText("Create User"));

      await waitFor(() => {
        expect(mockToastStore.error).toHaveBeenCalledWith(
          "Failed to create user",
          expect.any(String)
        );
      });
    });
  });

  describe("Delete user flow", () => {
    it("deletes user after confirmation", async () => {
      const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(true);
      mockBackendClient.deleteUser.mockResolvedValue(undefined);

      render(() => <UsersPage />);

      await waitFor(() => {
        expect(screen.getByText("John Doe")).toBeInTheDocument();
      });

      const deleteButtons = screen.getAllByTitle("Delete user");
      fireEvent.click(deleteButtons[0]);

      await waitFor(() => {
        expect(mockBackendClient.deleteUser).toHaveBeenCalledWith(1);
        expect(mockToastStore.success).toHaveBeenCalledWith("User John Doe deleted");
      });

      confirmSpy.mockRestore();
    });

    it("does not delete user when confirmation is cancelled", async () => {
      const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(false);

      render(() => <UsersPage />);

      await waitFor(() => {
        expect(screen.getByText("John Doe")).toBeInTheDocument();
      });

      const deleteButtons = screen.getAllByTitle("Delete user");
      fireEvent.click(deleteButtons[0]);

      expect(mockBackendClient.deleteUser).not.toHaveBeenCalled();

      confirmSpy.mockRestore();
    });
  });

  describe("Regenerate key flow", () => {
    it("regenerates key and calls backend", async () => {
      const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(true);
      mockBackendClient.regenerateApiKey.mockResolvedValue({
        apiKey: "sk-new-regenerated-key",
        apiKeyPrefix: "sk-john",
      });

      render(() => <UsersPage />);

      await waitFor(() => {
        expect(screen.getByText("John Doe")).toBeInTheDocument();
      });

      const regenButtons = screen.getAllByTitle("Regenerate API key");
      fireEvent.click(regenButtons[0]);

      await waitFor(() => {
        expect(mockBackendClient.regenerateApiKey).toHaveBeenCalledWith(1);
      });

      await waitFor(() => {
        expect(mockToastStore.success).toHaveBeenCalledWith("API key regenerated");
      });

      confirmSpy.mockRestore();
    });
  });

  describe("Toggle enabled status", () => {
    it("toggles user enabled status", async () => {
      mockBackendClient.updateUser.mockResolvedValue({
        ...mockUsers[0],
        enabled: false,
      });

      render(() => <UsersPage />);

      await waitFor(() => {
        expect(screen.getByText("John Doe")).toBeInTheDocument();
      });

      const toggleButtons = screen.getAllByTitle("Disable user");
      fireEvent.click(toggleButtons[0]);

      await waitFor(() => {
        expect(mockBackendClient.updateUser).toHaveBeenCalledWith(1, {
          enabled: false,
        });
        expect(mockToastStore.success).toHaveBeenCalledWith("User disabled");
      });
    });
  });
});
