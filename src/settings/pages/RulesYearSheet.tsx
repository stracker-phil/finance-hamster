import { useEffect, useId, useState } from "react";
import { api, type JsonPreview, type RulesField } from "../../api";
import { Modal } from "../../components/Modal";
import { useT } from "../../i18n";
import { showError, undoToast } from "../../lib/undoToast";
import { useRulesOverview, useRulesYear } from "../../queries/taxYears";

/** One value; saved when it loses focus or on Return, only when it changed. */
function FieldRow({ year, field, editable }: { year: number; field: RulesField; editable: boolean }) {
  const [value, setValue] = useState(field.value);
  useEffect(() => setValue(field.value), [field.value]);
  const id = useId();
  const save = () => {
    const next = value.trim();
    if (next === field.value) return;
    api
      .rulesSetValue(year, field.path, next)
      .then(undoToast)
      .catch((e: unknown) => {
        setValue(field.value);
        showError(e);
      });
  };
  return (
    <div className="settings-row">
      <div className="settings-row__text">
        <label htmlFor={id}>{field.label}</label>
      </div>
      <div className="settings-row__control">
        <input
          id={id}
          className="settings-number rules-field"
          value={value}
          readOnly={!editable}
          inputMode={field.kind === "amount" ? "decimal" : "numeric"}
          onChange={(e) => setValue(e.target.value)}
          onBlur={save}
          onKeyDown={(e) => {
            if (e.key === "Enter") e.currentTarget.blur();
          }}
        />
        <span className="rules-unit">{field.unit ?? ""}</span>
      </div>
    </div>
  );
}

function JsonSheet({ year, onClose }: { year: number; onClose: () => void }) {
  const id = useId();
  const { taxRules: text, common } = useT().settings;
  const [json, setJson] = useState<string | null>(null);
  const [preview, setPreview] = useState<JsonPreview | null>(null);
  useEffect(() => {
    api.rulesJson(year).then(setJson).catch(showError);
  }, [year]);
  const check = () => {
    if (json !== null) api.rulesPreviewJson(year, json).then(setPreview).catch(showError);
  };
  const save = () => {
    if (json === null) return;
    api
      .rulesSetJson(year, json)
      .then((label) => {
        undoToast(label);
        onClose();
      })
      .catch(showError);
  };
  return (
    <Modal labelledBy={id} className="sheet rules-json" onClose={onClose}>
      <h2 id={id}>{text.jsonTitle(year)}</h2>
      <p>{text.jsonHint}</p>
      <textarea
        className="rules-json__text"
        spellCheck={false}
        aria-label={text.jsonTitle(year)}
        value={json ?? ""}
        onChange={(e) => {
          setJson(e.target.value);
          setPreview(null);
        }}
      />
      {preview && preview.problems.length > 0 && (
        <section aria-label={text.problems}>
          <h3 className="rules-json__heading">{text.problems}</h3>
          <ul className="rules-list rules-list--problems">
            {preview.problems.map((p) => (
              <li key={p}>{p}</li>
            ))}
          </ul>
        </section>
      )}
      {preview && preview.problems.length === 0 && (
        <section aria-label={text.changes}>
          <h3 className="rules-json__heading">{text.changes}</h3>
          {preview.changes.length === 0 ? (
            <p>{text.noChanges}</p>
          ) : (
            <ul className="rules-list">
              {preview.changes.map((c) => (
                <li key={c}>{c}</li>
              ))}
            </ul>
          )}
        </section>
      )}
      <div className="actions">
        <button type="button" className="btn" onClick={onClose}>
          {common.cancel}
        </button>
        <button type="button" className="btn" onClick={check} disabled={json === null}>
          {text.check}
        </button>
        <button type="button" className="btn btn--primary" onClick={save} disabled={!preview || preview.problems.length > 0}>
          {text.save}
        </button>
      </div>
    </Modal>
  );
}

/** One year's rules in plain words. */
export function RulesYearSheet({ year, onClose }: { year: number; onClose: () => void }) {
  const id = useId();
  const { data } = useRulesYear(year);
  const overview = useRulesOverview().data?.years.find((y) => y.year === year);
  const [editingJson, setEditingJson] = useState(false);
  const text = useT().settings.taxRules;
  if (editingJson) return <JsonSheet year={year} onClose={() => setEditingJson(false)} />;
  const useBuiltin = () => {
    api.rulesUseBuiltin(year).then(undoToast).catch(showError);
  };
  return (
    <Modal labelledBy={id} className="sheet rules-sheet" onClose={onClose}>
      <h2 id={id}>{text.sheetTitle(year)}</h2>
      {data && <p>{data.source === "own" ? text.own(data.basedOn) : text.builtin(data.ruleset)}</p>}
      <div className="rules-sheet__body">
        {data?.groups.map((group) => (
          <section key={group.title} className="settings-group" aria-label={group.title}>
            <h3>{group.title}</h3>
            <div className="settings-card">
              {group.fields.map((field) => (
                <FieldRow key={field.path} year={year} field={field} editable={data.editable} />
              ))}
              {group.facts.map((fact) => (
                <div key={fact} className="settings-row settings-row__desc">
                  {fact}
                </div>
              ))}
            </div>
          </section>
        ))}
        {data && data.sources.length > 0 && (
          <section className="settings-group" aria-label={text.sources}>
            <h3>{text.sources}</h3>
            <ul className="rules-sources">
              {data.sources.map((s) => (
                <li key={s}>{s}</li>
              ))}
            </ul>
          </section>
        )}
        {data?.editable && (
          <details className="rules-advanced">
            <summary>{text.advanced}</summary>
            <button type="button" className="btn btn--small" onClick={() => setEditingJson(true)}>
              {text.editJson}
            </button>
          </details>
        )}
      </div>
      <div className="actions">
        {data?.editable && data.source === "own" && overview?.hasBuiltin && (
          <button type="button" className="btn" onClick={useBuiltin}>
            {overview?.builtinNewer ? text.useBuiltin(overview.builtinNewer) : text.backToBuiltin}
          </button>
        )}
        <button type="button" className="btn btn--primary" onClick={onClose}>
          {text.done}
        </button>
      </div>
    </Modal>
  );
}
