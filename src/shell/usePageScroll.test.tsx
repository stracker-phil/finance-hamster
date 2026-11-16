import { act, fireEvent, render, screen } from "@testing-library/react";
import { useRef } from "react";
import { beforeEach, describe, expect, it } from "vitest";
import { HOME } from "../state/route";
import { useStore } from "../state/store";
import { usePageScroll } from "./usePageScroll";

function Page() {
  const page = useRef<HTMLElement>(null);
  usePageScroll(page);
  return (
    <section ref={page}>
      <div className="page-scroll" role="region" aria-label="Content" />
      <div role="region" aria-label="Sidebar" />
    </section>
  );
}

const subscriptions = { page: "subscriptions" } as const;

const scrollTo = (name: string, top: number) => {
  const element = screen.getByRole("region", { name });
  element.scrollTop = top;
  fireEvent.scroll(element);
  return element;
};

beforeEach(() => {
  useStore.setState({ route: HOME, back: [], forward: [], filter: "", scroll: 0 });
  useStore.getState().noteScroll(0);
});

describe("usePageScroll", () => {
  it("reports where the page's scroller was scrolled to, so leaving the page remembers it", () => {
    render(<Page />);

    scrollTo("Content", 250);
    act(() => useStore.getState().navigate(subscriptions));

    expect(useStore.getState().back).toEqual([{ route: HOME, filter: "", scroll: 250 }]);
  });

  it("ignores the scrolling of anything but the page's scroller", () => {
    render(<Page />);

    scrollTo("Sidebar", 90);
    act(() => useStore.getState().navigate(subscriptions));

    expect(useStore.getState().back[0].scroll).toBe(0);
  });

  it("scrolls the scroller back to where the page was left when Back returns to it", () => {
    render(<Page />);
    const content = scrollTo("Content", 250);
    act(() => useStore.getState().navigate(subscriptions));
    content.scrollTop = 0;

    act(() => useStore.getState().goBack());

    expect(useStore.getState().scroll).toBe(250);
    expect(content.scrollTop).toBe(250);
  });
});
