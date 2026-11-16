import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createRef, useState, type Ref } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api } from "../../api";
import { useStore } from "../../state/store";
import { PdfCoversProbe } from "../../test/PdfCoversProbe";
import { MenuHost, PopUpButton } from ".";

const mocked = vi.mocked(api, true);

const OPTIONS = [
  { value: "monthly", label: "Monthly" },
  { value: "quarterly", label: "Quarterly" },
  { value: "yearly", label: "Yearly" },
];

interface HarnessProps {
  onChange?: (value: string) => void;
  disabled?: boolean;
  label?: string;
  ref?: Ref<HTMLButtonElement>;
}

function Harness({ onChange, disabled, label, ref }: HarnessProps) {
  const [value, setValue] = useState("monthly");
  return (
    <>
      <label htmlFor="period">Period</label>
      <PopUpButton
        id="period"
        ref={ref}
        value={value}
        options={OPTIONS}
        disabled={disabled}
        aria-label={label}
        onChange={(next) => {
          onChange?.(next);
          setValue(next);
        }}
      />
      <MenuHost />
    </>
  );
}

const button = () => screen.getByRole("button", { name: "Period" });

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ menu: null, pdfCovers: [] });
});

describe("PopUpButton", () => {
  it("shows the current choice as its label", () => {
    render(<PopUpButton value="quarterly" options={OPTIONS} onChange={() => {}} />);

    expect(screen.getByRole("button", { name: "Quarterly" })).toBeTruthy();
  });

  it("announces a menu popup that is collapsed until opened", async () => {
    render(<Harness />);
    expect(button().getAttribute("aria-haspopup")).toBe("menu");
    expect(button().getAttribute("aria-expanded")).toBe("false");

    await userEvent.click(button());
    expect(button().getAttribute("aria-expanded")).toBe("true");

    await userEvent.keyboard("{Escape}");
    expect(button().getAttribute("aria-expanded")).toBe("false");
  });

  it("lets clicks on the PDF preview through while open and sends the uncovered state once closed", async () => {
    render(
      <>
        <PdfCoversProbe />
        <Harness />
      </>,
    );

    await userEvent.click(button());
    await waitFor(() => expect(mocked.pdfViewCover).toHaveBeenLastCalledWith(expect.objectContaining({ hidden: false, clickThrough: true })));

    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(mocked.pdfViewCover).toHaveBeenLastCalledWith({ hidden: false, holes: [], clickThrough: false }));
  });

  it("lists its options with the current one checked", async () => {
    render(<Harness />);

    await userEvent.click(button());

    const rows = screen.getAllByRole("menuitemradio");
    expect(rows.map((r) => r.textContent)).toEqual(["Monthly", "Quarterly", "Yearly"]);
    expect(rows.map((r) => r.getAttribute("aria-checked"))).toEqual(["true", "false", "false"]);
  });

  it("changes to the chosen option and shows it", async () => {
    const onChange = vi.fn();
    render(<Harness onChange={onChange} />);
    await userEvent.click(button());

    await userEvent.click(screen.getByRole("menuitemradio", { name: "Yearly" }));

    expect(onChange).toHaveBeenCalledExactlyOnceWith("yearly");
    expect(screen.queryByRole("menu")).toBeNull();
    expect(button().textContent).toContain("Yearly");
  });

  it("reports no change when the current option is chosen again", async () => {
    const onChange = vi.fn();
    render(<Harness onChange={onChange} />);
    await userEvent.click(button());

    await userEvent.click(screen.getByRole("menuitemradio", { name: "Monthly" }));

    expect(onChange).not.toHaveBeenCalled();
    expect(screen.queryByRole("menu")).toBeNull();
  });

  it("opens on ArrowDown", async () => {
    render(<Harness />);
    button().focus();

    await userEvent.keyboard("{ArrowDown}");

    expect(screen.getByRole("menu", { name: "Period" })).toBeTruthy();
  });

  it("does not open while disabled", async () => {
    render(<Harness disabled />);

    await userEvent.click(button());
    button().focus();
    await userEvent.keyboard("{ArrowDown}");

    expect(button()).toBeDisabled();
    expect(screen.queryByRole("menu")).toBeNull();
  });

  describe("naming", () => {
    it("is named by its label element, and so is its menu", async () => {
      render(<Harness />);

      await userEvent.click(button());

      expect(screen.getByRole("menu", { name: "Period" })).toBeTruthy();
    });

    it("prefers its aria-label for itself and its menu", async () => {
      render(<Harness label="Billing period" />);

      await userEvent.click(screen.getByRole("button", { name: "Billing period" }));

      expect(screen.getByRole("menu", { name: "Billing period" })).toBeTruthy();
    });
  });

  it("hands its button to a ref prop", () => {
    const ref = createRef<HTMLButtonElement>();
    render(<Harness ref={ref} />);

    expect(ref.current).toBe(button());
  });
});
