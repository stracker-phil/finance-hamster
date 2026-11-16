import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Segmented } from "./controls";

const options = [
  { value: "a", label: "Alpha" },
  { value: "b", label: "Beta" },
];

describe("Segmented", () => {
  it("shows the key of each segment in its tooltip with keyHints", () => {
    render(<Segmented label="Pick" options={options} value="a" onChange={() => {}} keyHints />);

    expect(screen.getAllByRole("button").map((b) => b.title)).toEqual(["Alpha (⌥1)", "Beta (⌥2)"]);
  });

  it("shows no tooltip without keyHints", () => {
    render(<Segmented label="Pick" options={options} value="a" onChange={() => {}} />);

    expect(screen.getAllByRole("button").map((b) => b.title)).toEqual(["", ""]);
  });
});
