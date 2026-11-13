import { vi } from "vitest";
import type { api as realApi } from "../api";

type Api = typeof realApi;

/** A mock of every `api` function with harmless defaults; tests override what they need. */
export function createApiMock(overrides: Partial<Api> = {}): Api {
  const api: Api = {
    takeLaunchEvents: vi.fn().mockResolvedValue([]),
    logFrontendError: vi.fn().mockResolvedValue(undefined),
    appInfo: vi.fn().mockResolvedValue({ name: "Hamster (Debug)", version: "0.1.0" }),
    listen: vi.fn().mockResolvedValue(() => {}),
  };
  return { ...api, ...overrides };
}
