import { ChevronLeft, ChevronRight } from "lucide-react";

interface Props {
  year: number;
  /** The years it can step to, oldest first. */
  years: number[];
  label: string;
  previousLabel: string;
  nextLabel: string;
  onChange: (year: number) => void;
}

/** `‹ 2026 ›`: steps to the neighbouring year in `years`; an arrow without one is disabled but keeps its slot. */
export function YearStepper({ year, years, label, previousLabel, nextLabel, onChange }: Props) {
  const previous = years.filter((y) => y < year).at(-1);
  const next = years.find((y) => y > year);
  const step = (target: number | undefined) => {
    if (target !== undefined) onChange(target);
  };
  return (
    <div className="year-stepper" role="group" aria-label={label}>
      <button className="icon-btn" disabled={previous === undefined} aria-label={previousLabel} onClick={() => step(previous)}>
        <ChevronLeft size={14} />
      </button>
      <span className="year-stepper__year">{year}</span>
      <button className="icon-btn" disabled={next === undefined} aria-label={nextLabel} onClick={() => step(next)}>
        <ChevronRight size={14} />
      </button>
    </div>
  );
}
