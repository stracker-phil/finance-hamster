import { vi } from "vitest";
import type { api as realApi } from "../api";
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
    listen: vi.fn().mockResolvedValue(() => {}),
  };
  return { ...api, ...overrides };
}
