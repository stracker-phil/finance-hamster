import { ChevronLeft, ChevronRight } from "lucide-react";
import { usePageKeys } from "../commands/pageKeys";

interface Props {
  year: number;
  /** The years it can step to, oldest first. */
  years: number[];
  label: string;
  previousLabel: string;
  nextLabel: string;
  onChange: (year: number) => void;
}

/** The years before and after `year` in `years` (any order). */
const neighbours = (year: number, years: number[]) => ({
  previous: Math.max(...years.filter((y) => y < year)),
  next: Math.min(...years.filter((y) => y > year)),
});

/** ← or H or P steps to the previous year in `years`, → or L or N to the next, while the page shows. */
export function useYearKeys(year: number, years: number[], onChange: (year: number) => void) {
  const { previous, next } = neighbours(year, years);
  const step = (target: number) => () => {
    if (Number.isFinite(target)) onChange(target);
  };
  usePageKeys({ arrowleft: step(previous), h: step(previous), p: step(previous), arrowright: step(next), l: step(next), n: step(next) });
}

/** `‹ 2026 ›`: steps to the neighbouring year in `years`; an arrow without one is disabled but keeps its slot. */
export function YearStepper({ year, years, label, previousLabel, nextLabel, onChange }: Props) {
  const { previous, next } = neighbours(year, years);
  useYearKeys(year, years, onChange);
  return (
    <div className="year-stepper" role="group" aria-label={label}>
      <button className="icon-btn" disabled={!Number.isFinite(previous)} aria-label={previousLabel} title={`${previousLabel} (←)`} onClick={() => onChange(previous)}>
        <ChevronLeft size={14} />
      </button>
      <span className="year-stepper__year">{year}</span>
      <button className="icon-btn" disabled={!Number.isFinite(next)} aria-label={nextLabel} title={`${nextLabel} (→)`} onClick={() => onChange(next)}>
        <ChevronRight size={14} />
      </button>
    </div>
  );
}
