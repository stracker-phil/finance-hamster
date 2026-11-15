import { useEffect, useRef, useState } from "react";
import { Car, Copy, Euro, FileText, Landmark, ListChecks, Trash2 } from "lucide-react";
import { api, type AttentionKind, type AttentionView, type KennzahlView, type MonthView, type QuestionView, type YearRow } from "../api";
import { attachInvoiceTo, startAddDocument } from "../documents/addFlow";
import { copyText } from "../lib/clipboard";
import { showError } from "../lib/undoToast";
import { useYearPage } from "../queries/year";
import { useYearCheck } from "../queries/yearCheck";
import { useStore } from "../state/store";
import { AddButton, AddEntry } from "../year/AddEntry";
import { YearMenu } from "../year/YearMenu";
import { attachEvidence, deleteEntry, openEvidence } from "../year/entryActions";
import { EmptyPage } from "./EmptyPage";
import { t, useT } from "../i18n";


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
  const text = useT().pages.year;
  return (
    <button className="btn btn--small" onClick={() => attachEvidence(year, entryId)}>
      {text.attachPdf}
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
  const text = useT().pages.year;
  return (
    <div className="year-row-wrap">
      <button className="year-row" onClick={onOpen} disabled={!onOpen}>
        <Icon size={15} strokeWidth={1.8} className="docs-row__icon" />
        <span className="year-row__title">{row.title}</span>
        {row.draft && <span className="pill pill--dashed">{text.draft}</span>}
        {row.carriedFrom && <span className="pill">{text.carriedFrom(row.carriedFrom)}</span>}
        <span className="add-doc__hint year-row__detail">{row.detail}</span>
        {withAmount && <span className="year-row__amount">{row.amount}</span>}
      </button>
      {isEntry && open && (
        <span className="year-row__actions">
          {row.attach && <AttachButton year={year} entryId={row.id} />}
          <button className="icon-btn" aria-label={text.delete(row.title)} onClick={() => deleteEntry(year, row.id)}>
            <Trash2 size={14} />
          </button>
        </span>
      )}
    </div>
  );
}

function Kennzahl({ item, year, open }: { item: KennzahlView; year: number; open: boolean }) {
  const [adding, setAdding] = useState(false);
  const text = useT().pages.year;
  const copy = () =>
    copyText(item.copy)
      .then(() => useStore.getState().showToast("success", t().pages.year.copied(item.copy, item.kz)))
      .catch(showError);
  return (
    <section className={item.rows.length === 0 ? "kennzahl kennzahl--empty" : "kennzahl"} aria-label={`KZ ${item.kz} ${item.name}`}>
      <div className="kennzahl__head">
        <span className="mono kennzahl__kz">KZ {item.kz}</span>
        <span className="kennzahl__name">{item.name}</span>
        {open && <AddButton item={item} adding={adding} onToggle={() => setAdding((a) => !a)} />}
        {item.reported ? (
          <span className="pill">{text.notCounted}</span>
        ) : (
          <button className="copy-btn" onClick={() => void copy()} aria-label={text.copy(item.copy, item.kz)}>
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

/** A missing invoice's Attach PDF…: for its bank line when it has one, else any new document. */
const attachMissing = (item: AttentionView) => (item.lineId ? void attachInvoiceTo(item.lineId) : startAddDocument());

/** What needs attention; with `focus` only that kind, all of it, scrolled into view. */
function AttentionList({ year, items, focus }: { year: number; items: AttentionView[]; focus?: AttentionKind }) {
  const navigate = useStore((s) => s.navigate);
  const [all, setAll] = useState(false);
  const [only, setOnly] = useState(focus);
  const section = useRef<HTMLElement>(null);
  const text = useT().pages.year;
  useEffect(() => {
    if (focus) section.current?.scrollIntoView({ block: "start" });
  }, [focus]);
  if (items.length === 0) return null;
  const kept = only ? items.filter((i) => i.kind === only) : items;
  const shown = all || only ? kept : kept.slice(0, ATTENTION_SHOWN);
  const showEverything = () => {
    setOnly(undefined);
    setAll(true);
  };
  return (
    <section ref={section} className="year-attention" aria-label={text.needsAttention}>
      <div className="year-heading-row">
        <h3 className="year-heading">{only ? text.headings[only] : text.needsAttention}</h3>
        {only && kept.length < items.length && (
          <button className="link-btn year-heading__more" onClick={showEverything}>
            {text.showAllAttention(items.length)}
          </button>
        )}
      </div>
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
                {item.kind === "draft" ? text.openDraft : text.open}
              </button>
            ) : entryId ? (
              <AttachButton year={year} entryId={entryId} />
            ) : item.kind === "changed_since_closing" ? null : (
              <button className="btn btn--small" onClick={() => attachMissing(item)}>
                {text.attachPdf}
              </button>
            )}
          </div>
        );
      })}
      {!only && items.length > ATTENTION_SHOWN && (
        <button className="link-btn year-more" onClick={() => setAll((open) => !open)}>
          {all ? text.showFewer : text.showAll(items.length)}
        </button>
      )}
    </section>
  );
}

function MonthList({ year, months, open }: { year: number; months: MonthView[]; open: boolean }) {
  const text = useT().pages.year;
  if (months.length === 0) return <p className="add-doc__hint">{text.nothingCounts(year)}</p>;
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

/** "Year check: last run …" with its button, on an open year. */
function CheckLine({ year }: { year: number }) {
  const { data } = useYearCheck(year);
  const navigate = useStore((s) => s.navigate);
  const text = useT().pages.year;
  if (!data?.open) return null;
  return (
    <div className="check-line">
      <ListChecks size={15} />
      <span>{data.line}</span>
      <button className="btn btn--small" onClick={() => navigate({ page: "yearCheck", year })}>
        {text.startCheck}
      </button>
    </div>
  );
}

export function YearPage({ year, attention }: { year: number; attention?: AttentionKind }) {
  const filter = useStore((s) => s.filter);
  const [view, setView] = useState<"kennzahl" | "month">("kennzahl");
  const { data, isError } = useYearPage(year, filter);
  const text = useT().pages.year;
  if (isError) return <EmptyPage title={text.missingTitle(year)} text={text.missingText} />;
  if (!data) return null;
  const open = data.state === "open";
  return (
    <div className="page-scroll">
      <div className="year-page">
        <div className="year-hero">
          <div className="year-hero__main">
            <div className="add-doc__hint">
              {text.totalFor(data.year, data.ruleset)}
              {!open && <span>{text.closed}</span>}
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
          <YearMenu year={data.year} archived={data.archived} closed={!open} />
        </div>

        {open && <CheckLine year={data.year} />}

        {open && data.questions.length > 0 && (
          <section aria-label={text.questionsFor(data.year)}>
            <h3 className="year-heading">{text.questionsFor(data.year)}</h3>
            {data.questions.map((q) => (
              <QuestionRow key={q.id} year={data.year} question={q} />
            ))}
          </section>
        )}

        <AttentionList key={attention ?? "all"} year={data.year} items={data.attention} focus={attention} />

        <div className="year-switch" role="group" aria-label={text.show}>
          <button className="seg-btn" aria-pressed={view === "kennzahl"} onClick={() => setView("kennzahl")}>
            {text.byKennzahl}
          </button>
          <button className="seg-btn" aria-pressed={view === "month"} onClick={() => setView("month")}>
            {text.byMonth}
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
