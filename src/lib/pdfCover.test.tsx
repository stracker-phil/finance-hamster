import { render, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { useStore } from "../state/store";
import { useCoversPdf, usePdfCovers } from "./pdfCover";

const mocked = vi.mocked(api, true);

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ pdfCovers: 0 });
});

const covers = () => useStore.getState().pdfCovers;

function Overlay({ active }: { active?: boolean }) {
  useCoversPdf(active);
  return null;
}

function Overlays({ count }: { count: number }) {
  usePdfCovers();
  return (
    <>
      {Array.from({ length: count }, (_, i) => (
        <Overlay key={i} />
      ))}
    </>
  );
}

describe("useCoversPdf", () => {
  it("counts the overlay while it is mounted", () => {
    const { unmount } = render(<Overlay />);
    expect(covers()).toBe(1);

    unmount();

    expect(covers()).toBe(0);
  });

  it("counts the overlay only while it is active", () => {
    const { rerender } = render(<Overlay active={false} />);
    expect(covers()).toBe(0);

    rerender(<Overlay active />);
    expect(covers()).toBe(1);

    rerender(<Overlay active={false} />);
    expect(covers()).toBe(0);
  });

  it("counts every open overlay", () => {
    render(
      <>
        <Overlay />
        <Overlay />
      </>,
    );

    expect(covers()).toBe(2);
  });
});

describe("usePdfCovers", () => {
  it("leaves the preview alone while no overlay is open", () => {
    render(<Overlays count={0} />);

    expect(mocked.pdfViewSuspend).not.toHaveBeenCalled();
    expect(mocked.pdfViewResume).not.toHaveBeenCalled();
  });

  it("suspends the preview when the first overlay opens and resumes it when the last one closes", async () => {
    const { rerender } = render(<Overlays count={0} />);

    rerender(<Overlays count={1} />);
    await waitFor(() => expect(mocked.pdfViewSuspend).toHaveBeenCalledTimes(1));
    expect(mocked.pdfViewResume).not.toHaveBeenCalled();

    rerender(<Overlays count={0} />);
    await waitFor(() => expect(mocked.pdfViewResume).toHaveBeenCalledTimes(1));
  });

  it("suspends and resumes once for two overlays", async () => {
    const { rerender } = render(<Overlays count={0} />);

    rerender(<Overlays count={2} />);
    rerender(<Overlays count={1} />);
    expect(mocked.pdfViewResume).not.toHaveBeenCalled();
    rerender(<Overlays count={0} />);

    await waitFor(() => expect(mocked.pdfViewResume).toHaveBeenCalledTimes(1));
    expect(mocked.pdfViewSuspend).toHaveBeenCalledTimes(1);
  });

  it("logs the error when the preview cannot be suspended", async () => {
    mocked.pdfViewSuspend.mockRejectedValue({ kind: "internal", message: "No view", code: null });

    render(<Overlays count={1} />);

    await waitFor(() => expect(mocked.logFrontendError).toHaveBeenCalledWith("PDF preview: No view"));
  });
});
