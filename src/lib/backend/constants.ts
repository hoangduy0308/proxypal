// Constants re-exported from original tauri.ts for compatibility

import type { AmpModelSlot } from "./types";

export const AMP_MODEL_SLOTS: AmpModelSlot[] = [
  {
    id: "opus-4-5",
    name: "Smart",
    fromModel: "claude-opus-4-5-20251101",
    fromLabel: "Claude Opus 4.5 (200K)",
  },
  {
    id: "sonnet-4-5",
    name: "Librarian",
    fromModel: "claude-sonnet-4-5-20250929",
    fromLabel: "Claude Sonnet 4.5 (1M)",
  },
  {
    id: "haiku-4-5",
    name: "Rush / Search",
    fromModel: "claude-haiku-4-5-20251001",
    fromLabel: "Claude Haiku 4.5",
  },
  {
    id: "oracle",
    name: "Oracle",
    fromModel: "gpt-5.1",
    fromLabel: "GPT-5.1",
  },
  {
    id: "review",
    name: "Review",
    fromModel: "gemini-2.5-flash-lite",
    fromLabel: "Gemini 2.5 Flash-Lite",
  },
  {
    id: "handoff",
    name: "Handoff",
    fromModel: "gemini-2.5-flash",
    fromLabel: "Gemini 2.5 Flash",
  },
];

export const AMP_MODEL_ALIASES: Record<string, string> = {
  "claude-opus-4.5": "claude-opus-4-5-20251101",
  "claude-opus-4-5": "claude-opus-4-5-20251101",
  "claude-haiku-4.5": "claude-haiku-4-5-20251001",
  "claude-haiku-4-5": "claude-haiku-4-5-20251001",
  "claude-sonnet-4.5": "claude-sonnet-4-5-20250929",
  "claude-sonnet-4-5": "claude-sonnet-4-5-20250929",
};

export const COPILOT_MODELS = {
  openai: [
    { id: "copilot-gpt-4.1", name: "GPT-4.1", status: "GA" },
    { id: "copilot-gpt-5", name: "GPT-5", status: "GA" },
    { id: "copilot-gpt-5-mini", name: "GPT-5 Mini", status: "GA" },
    { id: "copilot-gpt-5-codex", name: "GPT-5 Codex", status: "Preview" },
    { id: "copilot-gpt-5.1", name: "GPT-5.1", status: "Preview" },
    { id: "copilot-gpt-5.1-codex", name: "GPT-5.1 Codex", status: "Preview" },
    {
      id: "copilot-gpt-5.1-codex-mini",
      name: "GPT-5.1 Codex Mini",
      status: "Preview",
    },
    { id: "copilot-gpt-4o", name: "GPT-4o", status: "Legacy" },
    { id: "copilot-gpt-4", name: "GPT-4", status: "Legacy" },
    { id: "copilot-gpt-4-turbo", name: "GPT-4 Turbo", status: "Legacy" },
    { id: "copilot-o1", name: "O1", status: "Legacy" },
    { id: "copilot-o1-mini", name: "O1 Mini", status: "Legacy" },
  ],
  claude: [
    { id: "copilot-claude-haiku-4.5", name: "Claude Haiku 4.5", status: "GA" },
    { id: "copilot-claude-opus-4.1", name: "Claude Opus 4.1", status: "GA" },
    { id: "copilot-claude-sonnet-4", name: "Claude Sonnet 4", status: "GA" },
    {
      id: "copilot-claude-sonnet-4.5",
      name: "Claude Sonnet 4.5",
      status: "GA",
    },
    {
      id: "copilot-claude-opus-4.5",
      name: "Claude Opus 4.5",
      status: "Preview",
    },
  ],
  gemini: [
    { id: "copilot-gemini-2.5-pro", name: "Gemini 2.5 Pro", status: "GA" },
    { id: "copilot-gemini-3-pro", name: "Gemini 3 Pro", status: "Preview" },
  ],
  other: [
    {
      id: "copilot-grok-code-fast-1",
      name: "Grok Code Fast 1 (xAI)",
      status: "GA",
    },
    {
      id: "copilot-raptor-mini",
      name: "Raptor Mini (Fine-tuned)",
      status: "Preview",
    },
  ],
};
