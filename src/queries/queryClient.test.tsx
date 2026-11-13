import { QueryClientProvider, useQuery } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useStore } from "../state/store";
import { createQueryClient } from "./queryClient";

const failing = vi.fn().mockRejectedValue({ kind: "io", message: "Disk full", code: null });

function Probe({ quiet }: { quiet: boolean }) {
  const { isError } = useQuery({ queryKey: ["probe"], queryFn: failing, meta: { quiet } });
  return <p>{isError ? "Failed" : "Loading"}</p>;
}

const renderProbe = (quiet: boolean) =>
  render(
    <QueryClientProvider client={createQueryClient()}>
      <Probe quiet={quiet} />
    </QueryClientProvider>,
  );

beforeEach(() => {
  useStore.setState({ toasts: [] });
});

describe("createQueryClient", () => {
  it("toasts the message of a failed read", async () => {
    renderProbe(false);

    await screen.findByText("Failed");
    expect(useStore.getState().toasts.map((t) => [t.kind, t.message])).toEqual([["error", "Disk full"]]);
  });

  it("shows no toast for a failed read whose page shows the failure itself", async () => {
    renderProbe(true);

    await screen.findByText("Failed");
    expect(useStore.getState().toasts).toEqual([]);
  });
});
