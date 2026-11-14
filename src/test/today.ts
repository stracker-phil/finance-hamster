import type { Step, TodayView, UpcomingCharge, YearSummary } from "../api";

/** A demo step (the headline or a quiet row), with `patch` applied. */
export function step(patch: Partial<Step> = {}): Step {
  return {
    key: "review",
    title: "12 lines to review",
    detail: "about 2 minutes",
    line: "Decide what each one is: ANV, subscription or ignore.",
    button: "Start review",
    target: { kind: "review" },
    ...patch,
  };
}

/** A demo upcoming subscription charge, with `patch` applied. */
export function upcomingCharge(patch: Partial<UpcomingCharge> = {}): UpcomingCharge {
  return { id: "sub-1", name: "Lumen AI", letter: "L", tone: 2, anv: false, when: "in 12 days", soon: false, amount: "€20.00", ...patch };
}

/** A demo tax year summary, with `patch` applied. */
export function yearSummary(patch: Partial<YearSummary> = {}): YearSummary {
  return { year: 2026, total: "€1,228.40", progress: "4 of 6 items answered", progressPercent: 66, ...patch };
}

/** A demo Today view with nothing open, with `patch` applied. */
export function todayView(patch: Partial<TodayView> = {}): TodayView {
  return { date: "Friday, 13 November", next: null, also: [], upcoming: [], year: null, statements: [], ...patch };
}
