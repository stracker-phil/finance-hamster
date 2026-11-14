import { useEffect, useState } from "react";
import { Check } from "lucide-react";
import { api, type CheckQuestion, type YearCheckView } from "../api";
import { usePageKeys } from "../commands/pageKeys";
import { showError } from "../lib/undoToast";
import { useYearCheck } from "../queries/yearCheck";
import { useStore } from "../state/store";
import { startDocument } from "../year/AddEntry";
import { CheckRow, type Added } from "../yearcheck/CheckRow";
import { EmptyPage } from "./EmptyPage";

const STACK = 5;

const keysOf = (questions: CheckQuestion[]) => questions.map((q) => q.key);

function Done({ year, view, answered, onAgain }: { year: number; view: YearCheckView; answered: number; onAgain: () => void }) {
  const navigate = useStore((s) => s.navigate);
  return (
    <div className="empty-page check-done">
      <span className="review-done__tile">
        <Check size={22} strokeWidth={2.6} />
      </span>
      <div className="empty-page__title">Year check done</div>
      <div>
        Checked on {view.lastRun} · {answered} {answered === 1 ? "question" : "questions"} answered
      </div>
      {!view.yearOver && <div className="check-done__note">{year} isn’t over yet. Run the check again after 31 Dec; Close Year will remind you.</div>}
      <div className="review-done__actions">
        <button className="btn" onClick={onAgain}>
          Run Again
        </button>
        <button className="btn btn--primary" onClick={() => navigate({ page: "year", year })}>
          Back to Tax Year {year}
        </button>
      </div>
    </div>
  );
}

/** The year check: the ruleset's questions as a stack; the run (answered, Later) lives only while the page shows. */
export function YearCheckPage({ year }: { year: number }) {
  const { data } = useYearCheck(year);
  const [queue, setQueue] = useState<string[] | null>(null);
  const [answered, setAnswered] = useState(0);
  const [focus, setFocus] = useState(0);
  const [added, setAdded] = useState<Record<string, Added>>({});
  const [inline, setInline] = useState<string | null>(null);
  const [finished, setFinished] = useState<YearCheckView | null>(null);

  useEffect(() => {
    if (data && queue === null) setQueue(keysOf(data.questions));
  }, [data, queue]);

  const byKey = new Map((data?.questions ?? []).map((q) => [q.key, q]));
  const open = (queue ?? []).flatMap((k) => byKey.get(k) ?? []);
  const shown = open.slice(0, STACK);
  const focused = Math.max(0, Math.min(focus, shown.length - 1));

  const forget = (key: string) => {
    setAdded(({ [key]: _, ...rest }) => rest);
    if (inline === key) setInline(null);
  };
  const answer = (key: string) => {
    const rest = keysOf(open).filter((k) => k !== key);
    setQueue(rest);
    setAnswered((n) => n + 1);
    forget(key);
    if (rest.length === 0) api.yearCheckComplete(year).then(setFinished).catch(showError);
  };
  const later = (key: string) => {
    setQueue([...keysOf(open).filter((k) => k !== key), key]);
    forget(key);
  };
  const onAdded = (key: string) => (label: string) => setAdded((m) => ({ ...m, [key]: { count: (m[key]?.count ?? 0) + 1, label } }));
  const yes = (q: CheckQuestion) => {
    if (q.add === "document") void startDocument(q.key, onAdded(q.key));
    else if (q.add) setInline(q.key);
  };
  const again = () => {
    setQueue(keysOf(data?.questions ?? []));
    setAnswered(0);
    setAdded({});
    setInline(null);
    setFinished(null);
    setFocus(0);
  };

  const current = shown[focused];
  const canAdd = current && current.add !== null && !added[current.key];
  usePageKeys({
    ...(canAdd ? { y: () => yes(current), n: () => answer(current.key) } : {}),
    ...(current ? { l: () => later(current.key) } : {}),
    arrowdown: () => setFocus(Math.min(focused + 1, shown.length - 1)),
    arrowup: () => setFocus(Math.max(focused - 1, 0)),
  });

  if (!data || queue === null) return null;
  if (finished) return <Done year={year} view={finished} answered={answered} onAgain={again} />;
  if (!data.open) return <EmptyPage title={`Tax year ${year} is closed`} text="Reopen it to run the year check." />;
  if (data.questions.length === 0)
    return <EmptyPage title="Nothing to ask" text={`No item in rules-${year}.json has an ask block. Add them to the rules file, then Reload in Settings → Tax Rules.`} />;
  return (
    <div className="page-scroll check">
      <div className="check__column">
        <div className="check__head">
          <div>
            <h2 className="check__title">
              Year check {year} · {open.length} open
            </h2>
            <div className="check__sub">Things you might forget to claim. Say No to what doesn’t apply. Later moves a question to the end.</div>
          </div>
          <span className="review__keys">
            <span className="kbd">Y</span> <span className="kbd">N</span> <span className="kbd">L</span>
          </span>
        </div>
        <div className="check__stack">
          {shown.map((q, i) => (
            <CheckRow
              key={q.key}
              year={year}
              question={q}
              focused={i === focused}
              added={added[q.key]}
              adding={inline === q.key}
              onFocus={() => setFocus(i)}
              onYes={() => yes(q)}
              onAnswer={() => answer(q.key)}
              onLater={() => later(q.key)}
              onAdded={onAdded(q.key)}
              onCloseInput={() => setInline(null)}
            />
          ))}
        </div>
        {open.length > STACK && <div className="check__more">{open.length - STACK} more below, they move up as you answer</div>}
      </div>
    </div>
  );
}
