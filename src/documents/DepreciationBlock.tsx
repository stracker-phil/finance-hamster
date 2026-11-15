import { PopUpButton } from "../components/menu";
import { useId } from "react";
import type { DepreciationView } from "../api";
import { useT } from "../i18n";

const YEARS = Array.from({ length: 15 }, (_, i) => i + 1);

/** A purchase above the GWG limit: its cost, the useful life (the rule's or the document's own), and its slices. */
export function DepreciationBlock({ depreciation, disabled, onYears }: { depreciation: DepreciationView; disabled: boolean; onYears: (years: number | null) => void }) {
  const id = useId();
  const text = useT().documents.depreciation;
  return (
    <section className="depreciation" aria-label={text.title}>
      <h3 className="doc-form__heading">{text.title}</h3>
      <div className="doc-form__grid">
        <label htmlFor={`${id}-life`}>{text.usefulLife}</label>
        <div className="doc-form__row">
          <PopUpButton
            id={`${id}-life`}
            className="depreciation__life"
            value={depreciation.own ? depreciation.years : 0}
            disabled={disabled}
            options={[{ value: 0, label: text.asRules }, ...YEARS.map((y) => ({ value: y, label: text.years(y) }))]}
            onChange={(years) => onYears(years || null)}
          />
          <span className="add-doc__hint">{depreciation.rule}</span>
        </div>
        <span>{text.why}</span>
        <span className="depreciation__why">{depreciation.why}</span>
      </div>
      <ol className="depreciation__slices" aria-label={text.slices}>
        {depreciation.slices.map((slice) => (
          <li key={slice.year} className={slice.note === "frozen" ? "slice slice--frozen" : "slice"}>
            <span className="slice__year">{slice.year}</span>
            <span className="slice__amount">{slice.amount}</span>
            <span className="add-doc__hint">{slice.note ?? ""}</span>
          </li>
        ))}
      </ol>
    </section>
  );
}
