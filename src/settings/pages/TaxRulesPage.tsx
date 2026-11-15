import { useId, useState } from "react";
import { api, toAppError, type ReloadPreview, type RulesFile, type RulesState } from "../../api";
import { Modal } from "../../components/Modal";
import { t, useT } from "../../i18n";
import { useDataFolder } from "../../queries/settings";
import { useRulesOverview } from "../../queries/taxYears";
import { useStore } from "../../state/store";
import { Group, Row } from "../controls";
import { ReloadSheet } from "./ReloadSheet";

const PILLS: Record<RulesState, string> = {
  draft: "pill",
  open: "pill pill--blue",
  closed: "pill pill--green",
};

const EXCERPT = `{
  "schemaVersion": 1,
  "ruleset": "2026 v1",
  "year": 2026,
  "settings": { "gwgLimit": 1000.00, "halfYearFrom": "07-01" },
  "items": [
    {
      "key": "digital-equipment",
      "kz": "169",
      "name": "Digitale Arbeitsmittel",
      "section": "werbungskosten",
      "kind": "documents",
      "privateShare": 40,
      "estimate": [{ "type": "reduceBy", "question": "telearbeitspauschale" }]
    }
  ]
}`;

const showError = (e: unknown) => useStore.getState().showToast("error", toAppError(e).message);

const problemText = (problems: string[]) => t().settings.taxRules.cantBeUsed(problems[0], problems.length - 1);

/** The line under a file: what it is for, and whether it can be used. */
export function describeFile(file: RulesFile): string {
  const text = t().settings.taxRules;
  const parts: string[] = [];
  if (file.state === "draft") parts.push(file.basedOn ? text.duplicatedFrom(file.basedOn) : text.nextYear, text.notUsedYet);
  if (file.state === "open") {
    if (file.missing) return text.missing;
    parts.push(text.taxYear(file.year));
    if (file.problems.length === 0) parts.push(text.valid, text.items(file.items));
    if (file.unverified > 0) parts.push(text.unverified(file.unverified));
    if (file.edited && file.problems.length === 0) parts.push(text.edited);
  }
  if (file.state === "closed") parts.push(text.frozen(file.year, file.closedOn), text.readOnly);
  if (file.problems.length > 0 && file.state !== "closed") parts.push(problemText(file.problems));
  return parts.join(" · ");
}

function reloadText(files: RulesFile[]): string {
  const open = files.filter((f) => f.state === "open" && !f.missing);
  const broken = open.find((f) => f.problems.length > 0);
  if (broken) return `${broken.fileName} ${problemText(broken.problems)}`;
  const edited = open.find((f) => f.edited);
  const text = t().settings.taxRules;
  if (edited) return text.wasEdited(edited.fileName);
  return text.lastLoaded(open[0]?.loadedOn ?? null);
}

function DuplicateSheet({ from, onClose }: { from: number; onClose: () => void }) {
  const id = useId();
  const { taxRules: text, common } = useT().settings;
  const duplicate = () => {
    api
      .rulesDuplicate()
      .then((name) => useStore.getState().showToast("success", t().settings.taxRules.created(name)))
      .catch(showError)
      .finally(onClose);
  };
  return (
    <Modal labelledBy={id} className="sheet--narrow" onClose={onClose}>
      <h2 id={id}>{text.startTitle(from + 1)}</h2>
      <p>{text.startText(from)}</p>
      <div className="actions">
        <button type="button" className="btn" onClick={onClose}>
          {common.cancel}
        </button>
        <button type="button" className="btn btn--primary" onClick={duplicate}>
          {text.duplicateAction}
        </button>
      </div>
    </Modal>
  );
}

/** Settings → Tax Rules: the files with their state, Reload, Import, Duplicate. */
export function TaxRulesPage() {
  const { data: files = [], refetch } = useRulesOverview();
  const { data: folder } = useDataFolder();
  const [previews, setPreviews] = useState<ReloadPreview[] | null>(null);
  const [duplicating, setDuplicating] = useState(false);
  const newestOpen = files.find((f) => f.state === "open")?.year;
  const nextExists = newestOpen !== undefined && files.some((f) => f.year === newestOpen + 1);
  const { taxRules: text, common } = useT().settings;

  const reload = () => {
    api
      .rulesReloadPreview()
      .then((found) => {
        if (found.length > 0) setPreviews(found);
        else useStore.getState().showToast("info", t().settings.taxRules.noChange);
      })
      .catch(showError);
  };
  const importFile = () => {
    api
      .rulesImport()
      .then((imported) => {
        if (!imported) return;
        useStore.getState().showToast("success", t().settings.taxRules.imported(imported.fileName, imported.backup));
        void refetch();
        if (imported.needsReload) reload();
      })
      .catch(showError);
  };

  return (
    <>
      <Group title={text.rulesets} anchor="rulesets">
        {files.map((file) => (
          <Row key={file.year} label={file.fileName} description={describeFile(file)}>
            <span className={PILLS[file.state]}>{text.pills[file.state]}</span>
            <button className="btn btn--small" disabled={file.missing && file.state !== "closed"} onClick={() => void api.rulesOpen(file.year).catch(showError)}>
              {file.state === "closed" ? text.view : text.openInEditor}
            </button>
          </Row>
        ))}
      </Group>
      <Group title={text.actions} anchor="rules-actions">
        <Row label={text.reload} description={reloadText(files)} anchor="reload">
          <button className="btn btn--small" onClick={reload}>
            {text.reloadButton}
          </button>
        </Row>
        <Row label={text.import} description={text.importDescription} anchor="import">
          <button className="btn btn--small" onClick={importFile}>
            {text.importButton}
          </button>
        </Row>
        <Row
          label={text.duplicate}
          description={newestOpen === undefined ? text.needsOpenYear : nextExists ? text.nextExists(newestOpen + 1) : text.copies(newestOpen)}
          anchor="duplicate"
        >
          <button className="btn btn--small" disabled={newestOpen === undefined || nextExists} onClick={() => setDuplicating(true)}>
            {text.duplicateButton}
          </button>
        </Row>
        <Row label={text.folder} description={<span className="mono">{folder ? `${folder}rules/` : ""}</span>} anchor="rules-folder">
          <button className="btn btn--small" onClick={() => void api.rulesShowFolder().catch(showError)}>
            {common.showInFinder}
          </button>
        </Row>
      </Group>
      <Group title={text.format} anchor="rules-format">
        <Row label={text.excerpt} description={text.excerptDescription} />
        <pre className="settings-code">{EXCERPT}</pre>
      </Group>
      {previews && <ReloadSheet previews={previews} onClose={() => setPreviews(null)} />}
      {duplicating && newestOpen !== undefined && (
        <DuplicateSheet
          from={newestOpen}
          onClose={() => {
            setDuplicating(false);
            void refetch();
          }}
        />
      )}
    </>
  );
}
