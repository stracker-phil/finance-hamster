import { ChevronRight } from "lucide-react";
import type { StatementReach, Step, StepTarget, UpcomingCharge, YearSummary } from "../api";
import hamster from "../assets/hamster.svg";
import { accountTone } from "../lib/accountTone";
import { useToday } from "../queries/today";
import { startImport } from "../statements/importFlow";
import { useStore } from "../state/store";

function go(target: StepTarget) {
  const { navigate } = useStore.getState();
  switch (target.kind) {
    case "review":
      return navigate({ page: "review" });
    case "year":
      return navigate({ page: "year", year: target.year });
    case "import":
      return startImport();
    case "document":
      return navigate({ page: "document", documentId: target.id, year: target.year });
  }
}

function QuietRow({ step }: { step: Step }) {
  return (
    <button className="today-row" onClick={() => go(step.target)}>
      <span className={`year-dot today-dot--${step.key}`} />
      <span className="today-row__title">{step.title}</span>
      <span className="add-doc__hint">{step.detail}</span>
      <ChevronRight size={14} className="docs-row__icon" />
    </button>
  );
}

function NextStep({ next }: { next: Step | null }) {
  if (!next)
    return (
      <div className="today__done">
        <img src={hamster} alt="" className="today__hamster" />
        <div>
          <h2 className="today__headline">You’re up to date.</h2>
          <p className="today__line">Nothing to review or attach.</p>
        </div>
      </div>
    );
  return (
    <>
      <h2 className="today__headline">{next.title}</h2>
      <p className="today__line">{next.line}</p>
      <button className="btn btn--primary today__button" onClick={() => go(next.target)}>
        {next.button}
      </button>
    </>
  );
}

function UpcomingCharges({ charges }: { charges: UpcomingCharge[] }) {
  return (
    <section className="today__upcoming" aria-label="Coming up">
      <h3 className="sub-sheet__heading today__heading">
        Coming up
        <button className="link-btn today__more" onClick={() => useStore.getState().navigate({ page: "subscriptions" })}>
          All subscriptions
        </button>
      </h3>
      {charges.length === 0 && <p className="add-doc__hint">No subscription charges ahead.</p>}
      {charges.map((charge) => (
        <div key={charge.id} className="sub-line">
          <span className={`tile tile--small ${accountTone(charge.tone)}`}>{charge.letter}</span>
          <span className="sub-line__main today__name">
            {charge.name}
            {charge.anv && <span className="pill pill--anv">ANV</span>}
          </span>
          <span className={charge.soon ? "today__when today__when--soon" : "today__when"}>{charge.when}</span>
          <span className="year-row__amount today__amount">{charge.amount}</span>
        </div>
      ))}
    </section>
  );
}

function YearColumn({ year, statements }: { year: YearSummary | null; statements: StatementReach[] }) {
  return (
    <section className="today__year" aria-label={year ? `Tax year ${year.year}` : "Statements"}>
      {year && (
        <>
          <h3 className="sub-sheet__heading today__heading">
            Tax year {year.year}
            <button className="link-btn today__more" onClick={() => useStore.getState().navigate({ page: "year", year: year.year })}>
              Open
            </button>
          </h3>
          <div className="today__total">{year.total}</div>
          <div className="add-doc__hint">Werbungskosten so far</div>
          <div className="year-bar today__bar">
            <span style={{ width: `${year.progressPercent}%` }} />
          </div>
          <div className="add-doc__hint">{year.progress}</div>
        </>
      )}
      {statements.length > 0 && (
        <p className="today__statements">
          Statements:{" "}
          {statements.map((part, i) => (
            <span key={i} className={part.behind ? "today__behind" : undefined}>
              {i > 0 && " · "}
              {part.text}
            </span>
          ))}
        </p>
      )}
    </section>
  );
}

/** The single most useful next step with one button, the other open ones, and what's coming up. */
export function TodayPage() {
  const { data } = useToday();
  if (!data) return null;
  return (
    <div className="page-scroll">
      <div className="today">
        <div className="today__date">{data.date}</div>
        <NextStep next={data.next} />
        {data.also.length > 0 && (
          <div className="today__also" aria-label="Also open" role="group">
            {data.also.map((step) => (
              <QuietRow key={step.key} step={step} />
            ))}
          </div>
        )}
        <div className="today__columns">
          <UpcomingCharges charges={data.upcoming} />
          <YearColumn year={data.year} statements={data.statements} />
        </div>
      </div>
    </div>
  );
}
