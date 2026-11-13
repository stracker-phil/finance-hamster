import { vi } from "vitest";
import type { api as realApi } from "../api";
import { account } from "./accounts";
import { settings } from "./settings";

type Api = typeof realApi;

/** A mock of every `api` function with harmless defaults; tests override what they need. */
export function createApiMock(overrides: Partial<Api> = {}): Api {
  const api: Api = {
    takeLaunchEvents: vi.fn().mockResolvedValue([]),
    logFrontendError: vi.fn().mockResolvedValue(undefined),
    appInfo: vi.fn().mockResolvedValue({ name: "Hamster (Debug)", version: "0.1.0" }),
    listCommands: vi.fn().mockResolvedValue([]),
    runCommand: vi.fn().mockResolvedValue(undefined),
    getSettings: vi.fn().mockResolvedValue(settings()),
    updateSettings: vi.fn().mockResolvedValue(settings()),
    showDataFolder: vi.fn().mockResolvedValue(undefined),
    dataFolder: vi.fn().mockResolvedValue("~/Library/Application Support/com.example.hamster/"),
    undo: vi.fn().mockResolvedValue(null),
    redo: vi.fn().mockResolvedValue(null),
    listAccounts: vi.fn().mockResolvedValue([account()]),
    addAccount: vi.fn().mockResolvedValue(account()),
    editAccount: vi.fn().mockResolvedValue(undefined),
    listTaxYears: vi.fn().mockResolvedValue([]),
    rulesOverview: vi.fn().mockResolvedValue([]),
    rulesReloadPreview: vi.fn().mockResolvedValue([]),
    rulesReloadApply: vi.fn().mockResolvedValue(null),
    rulesImport: vi.fn().mockResolvedValue(null),
    rulesDuplicate: vi.fn().mockResolvedValue("rules-2027.json"),
    rulesOpen: vi.fn().mockResolvedValue(undefined),
    rulesShowFolder: vi.fn().mockResolvedValue(undefined),
    listen: vi.fn().mockResolvedValue(() => {}),
  };
  return { ...api, ...overrides };
}
