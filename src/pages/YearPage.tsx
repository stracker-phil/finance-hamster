import { useEffect, useState } from "react";
import { Car, Copy, Euro, FileText, Landmark, MoreHorizontal, Trash2 } from "lucide-react";
import { api, toAppError, type AttentionView, type KennzahlView, type MonthView, type QuestionView, type YearRow } from "../api";
import { startAddDocument } from "../documents/addFlow";
import { copyText } from "../lib/clipboard";
import { useCoversPdf } from "../lib/pdfCover";
import { useYearPage } from "../queries/year";
import { useStore } from "../state/store";
import { AddButton, AddEntry } from "../year/AddEntry";
import { attachEvidence, deleteEntry, openEvidence } from "../year/entryActions";
import { EmptyPage } from "./EmptyPage";

const showError = (e: unknown) => useStore.getState().showToast("error", toAppError(e).message);

function QuestionRow({ year, question }: { year: number; question: QuestionView }) {
  const [text, setText] = useState(question.value);
  useEffect(() => setText(question.value), [question.value]);
  const save = () => {
    if (text !== question.value) api.yearAnswer(year, question.id, text).catch(showError);
  };
  return (
    <div className="year-line">
      <span className={question.answered ? "year-dot year-dot--done" : "year-dot"} />
      <span className="year-line__text">
        {question.label}
        {question.help && <span className="add-doc__hint year-line__help">{question.help}</span>}
      </span>
      <label className={question.answered ? "year-answer" : "year-answer year-answer--empty"}>
        <input
          value={text}
          inputMode="decimal"
          aria-label={question.label}
          onChange={(e) => setText(e.target.value)}
          onBlur={save}
          onKeyDown={(e) => {
            if (e.key === "Enter") e.currentTarget.blur();
          }}
        />
        {question.unit && <span>{question.unit}</span>}
      </label>
    </div>
  );
}

const ROW_ICONS = { document: FileText, line: Landmark, amount: Euro, trip: Car };

function AttachButton({ year, entryId }: { year: number; entryId: string }) {
  return (
    <button className="btn btn--small" onClick={() => attachEvidence(year, entryId)}>
      Attach PDF…
    </button>
  );
}

function Row({ row, year, withAmount, open }: { row: YearRow; year: number; withAmount: boolean; open: boolean }) {
  const navigate = useStore((s) => s.navigate);
  const isEntry = row.kind === "amount" || row.kind === "trip";
  const onOpen =
    row.kind === "document"
      ? () => navigate({ page: "document", documentId: row.id, year })
      : row.kind === "amount" && !row.attach
        ? () => openEvidence(year, row.id)
        : undefined;
  const Icon = ROW_ICONS[row.kind];
  return (
    <div className="year-row-wrap">
      <button className="year-row" onClick={onOpen} disabled={!onOpen}>
        <Icon size={15} strokeWidth={1.8} className="docs-row__icon" />
        <span className="year-row__title">{row.title}</span>
        {row.draft && <span className="pill pill--dashed">Draft</span>}
        {row.carriedFrom && <span className="pill">From {row.carriedFrom}</span>}
        <span className="add-doc__hint year-row__detail">{row.detail}</span>
        {withAmount && <span className="year-row__amount">{row.amount}</span>}
      </button>
      {isEntry && open && (
        <span className="year-row__actions">
          {row.attach && <AttachButton year={year} entryId={row.id} />}
          <button className="icon-btn" aria-label={`Delete ${row.title}`} onClick={() => deleteEntry(year, row.id)}>
            <Trash2 size={14} />
          </button>
        </span>
      )}
    </div>
  );
}

function Kennzahl({ item, year, open }: { item: KennzahlView; year: number; open: boolean }) {
  const [adding, setAdding] = useState(false);
  const copy = () =>
    copyText(item.copy)
      .then(() => useStore.getState().showToast("success", `Copied ${item.copy} for KZ ${item.kz}`))
      .catch(showError);
  return (
    <section className={item.rows.length === 0 ? "kennzahl kennzahl--empty" : "kennzahl"} aria-label={`KZ ${item.kz} ${item.name}`}>
      <div className="kennzahl__head">
        <span className="mono kennzahl__kz">KZ {item.kz}</span>
        <span className="kennzahl__name">{item.name}</span>
        {open && <AddButton item={item} adding={adding} onToggle={() => setAdding((a) => !a)} />}
        {item.reported ? (
          <span className="pill">Not counted</span>
        ) : (
          <button className="copy-btn" onClick={() => void copy()} aria-label={`Copy ${item.copy} for KZ ${item.kz}`}>
            {item.copy}
            <Copy size={13} />
          </button>
        )}
      </div>
      {item.rows.map((row) => (
        <Row key={row.id} row={row} year={year} withAmount={item.rows.length > 1} open={open} />
      ))}
      {open && adding && <AddEntry year={year} item={item} onDone={() => setAdding(false)} />}
      {item.note && <div className="kennzahl__note">{item.note}</div>}
    </section>
  );
}

/** Rows shown before "Show all": a long list hides the sections below it. */
const ATTENTION_SHOWN = 5;

function AttentionList({ year, items }: { year: number; items: AttentionView[] }) {
  const navigate = useStore((s) => s.navigate);
  const [all, setAll] = useState(false);
  if (items.length === 0) return null;
  const shown = all ? items : items.slice(0, ATTENTION_SHOWN);
  return (
    <section aria-label="Needs attention">
      <h3 className="year-heading">Needs attention</h3>
      {shown.map((item, i) => {
        const { documentId, entryId } = item;
        return (
          <div key={i} className="year-line">
            <span className="year-dot year-dot--warn" />
            <span className="year-line__text">
              {item.title}
              <span className="add-doc__hint year-line__help">{item.detail}</span>
            </span>
            {documentId ? (
              <button className="btn btn--small" onClick={() => navigate({ page: "document", documentId, year })}>
                {item.kind === "draft" ? "Open Draft" : "Open"}
              </button>
            ) : entryId ? (
              <AttachButton year={year} entryId={entryId} />
            ) : (
              <button className="btn btn--small" onClick={startAddDocument}>
                Attach PDF…
              </button>
            )}
          </div>
        );
      })}
      {items.length > ATTENTION_SHOWN && (
        <button className="link-btn year-more" onClick={() => setAll((open) => !open)}>
          {all ? "Show fewer" : `Show all ${items.length}`}
        </button>
      )}
    </section>
  );
}

function MonthList({ year, months, open }: { year: number; months: MonthView[]; open: boolean }) {
  if (months.length === 0) return <p className="add-doc__hint">Nothing counts in {year} yet.</p>;
  return (
    <>
      {months.map((month) => (
        <section key={month.label} className="kennzahl" aria-label={month.label}>
          <div className="kennzahl__head">
            <span className="kennzahl__name">{month.label}</span>
            <span className="year-row__amount">{month.total}</span>
          </div>
          {month.rows.map((row) => (
            <Row key={row.id} row={row} year={year} withAmount open={open} />
          ))}
        </section>
      ))}
    </>
  );
}

function YearMenu({ year, archived }: { year: number; archived: number }) {
  const [open, setOpen] = useState(false);
  const navigate = useStore((s) => s.navigate);
  useCoversPdf(open);
  const go = (fn: () => void) => () => {
    setOpen(false);
    fn();
  };
  return (
    <div className="year-menu">
      <button className="icon-btn" aria-label="More" aria-expanded={open} onClick={() => setOpen((o) => !o)}>
        <MoreHorizontal size={16} />
      </button>
      {open && (
        <div className="menu year-menu__list" role="menu">
          <button className="menu-row" role="menuitem" onClick={go(() => navigate({ page: "log", year }))}>
            Log
          </button>
          <button className="menu-row" role="menuitem" onClick={go(() => navigate({ page: "archive", year }))}>
            Archive
            <span className="menu-row__key">{archived}</span>
          </button>
          <button className="menu-row" role="menuitem" disabled>
            Export Summary PDF…
          </button>
          <button className="menu-row" role="menuitem" onClick={go(() => void api.rulesOpen(year).catch(showError))}>
            Open Ruleset JSON
          </button>
        </div>
      )}
    </div>
  );
}

export function YearPage({ year }: { year: number }) {
  const filter = useStore((s) => s.filter);
  const [view, setView] = useState<"kennzahl" | "month">("kennzahl");
  const { data, isError } = useYearPage(year, filter);
  if (isError) return <EmptyPage title={`No tax year ${year}`} text="Its rules file isn't loaded yet; see Settings → Tax Rules." />;
  if (!data) return null;
  const open = data.state === "open";
  return (
    <div className="page-scroll">
      <div className="year-page">
        <div className="year-hero">
          <div className="year-hero__main">
            <div className="add-doc__hint">
              Total to enter for {data.year} · ruleset {data.ruleset}
            </div>
            <div className="year-hero__total">{data.total}</div>
            {data.pauschaleNote && <div className="add-doc__hint">{data.pauschaleNote}</div>}
          </div>
          <div className="year-hero__progress">
            <div className="year-bar">
              <span style={{ width: `${data.progressPercent}%` }} />
            </div>
            <div className="add-doc__hint">{data.progress}</div>
          </div>
          <YearMenu year={data.year} archived={data.archived} />
        </div>

        {open && data.questions.length > 0 && (
          <section aria-label={`Questions for ${data.year}`}>
            <h3 className="year-heading">Questions for {data.year}</h3>
            {data.questions.map((q) => (
              <QuestionRow key={q.id} year={data.year} question={q} />
            ))}
          </section>
        )}

        <AttentionList year={data.year} items={data.attention} />

        <div className="year-switch" role="group" aria-label="Show">
          <button className="seg-btn" aria-pressed={view === "kennzahl"} onClick={() => setView("kennzahl")}>
            By Kennzahl
          </button>
          <button className="seg-btn" aria-pressed={view === "month"} onClick={() => setView("month")}>
            By month
          </button>
        </div>

        {view === "kennzahl" ? (
          data.kennzahlen.map((item) => <Kennzahl key={item.key} item={item} year={data.year} open={open} />)
        ) : (
          <MonthList year={data.year} months={data.months} open={open} />
        )}
      </div>
    </div>
  );
}
