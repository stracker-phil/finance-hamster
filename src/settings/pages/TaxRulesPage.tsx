import { useId, useState } from "react";
import { api, toAppError, type ReloadPreview, type RulesFile, type RulesState } from "../../api";
import { Modal } from "../../components/Modal";
import { useDataFolder } from "../../queries/settings";
import { useRulesOverview } from "../../queries/taxYears";
import { useStore } from "../../state/store";
import { Group, Row } from "../controls";
import { ReloadSheet } from "./ReloadSheet";

const PILLS: Record<RulesState, { label: string; className: string }> = {
  draft: { label: "Draft", className: "pill" },
  open: { label: "Open", className: "pill pill--blue" },
  closed: { label: "Closed", className: "pill pill--green" },
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

function problemText(problems: string[]) {
  const more = problems.length > 1 ? ` (and ${problems.length - 1} more)` : "";
  return `can't be used: ${problems[0]}${more}`;
}

/** The line under a file: what it is for, and whether it can be used. */
export function describeFile(file: RulesFile): string {
  const parts: string[] = [];
  if (file.state === "draft") parts.push(file.basedOn ? `Duplicated from ${file.basedOn}` : "Next year", "not used yet");
  if (file.state === "open") {
    if (file.missing) return "The file is missing; the year keeps the rules it loaded";
    parts.push(`Tax Year ${file.year}`);
    if (file.problems.length === 0) parts.push("valid", `${file.items} items`);
    if (file.unverified > 0) parts.push(`${file.unverified} unverified`);
    if (file.edited && file.problems.length === 0) parts.push("edited, reload to apply");
  }
  if (file.state === "closed") parts.push(`Frozen with Tax Year ${file.year}${file.closedOn ? ` on ${file.closedOn}` : ""}`, "read only");
  if (file.problems.length > 0 && file.state !== "closed") parts.push(problemText(file.problems));
  return parts.join(" · ");
}

function reloadText(files: RulesFile[]): string {
  const open = files.filter((f) => f.state === "open" && !f.missing);
  const broken = open.find((f) => f.problems.length > 0);
  if (broken) return `${broken.fileName} ${problemText(broken.problems)}`;
  const edited = open.find((f) => f.edited);
  if (edited) return `${edited.fileName} was edited. Shows what changes before applying.`;
  const loaded = open[0]?.loadedOn;
  return `${loaded ? `Last loaded ${loaded} · no errors. ` : ""}Shows what changes in the open year before applying.`;
}

function DuplicateSheet({ from, onClose }: { from: number; onClose: () => void }) {
  const id = useId();
  const duplicate = () => {
    api
      .rulesDuplicate()
      .then((name) => useStore.getState().showToast("success", `Created ${name} as a draft`))
      .catch(showError)
      .finally(onClose);
  };
  return (
    <Modal labelledBy={id} className="sheet--narrow" onClose={onClose}>
      <h2 id={id}>Start {from + 1}</h2>
      <p>
        Copies rules-{from}.json to rules-{from + 1}.json as a draft. Check every value against the new year&apos;s forms before the year opens.
      </p>
      <div className="actions">
        <button type="button" className="btn" onClick={onClose}>
          Cancel
        </button>
        <button type="button" className="btn btn--primary" onClick={duplicate}>
          Duplicate
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

  const reload = () => {
    api
      .rulesReloadPreview()
      .then((found) => {
        if (found.length > 0) setPreviews(found);
        else useStore.getState().showToast("info", "No rules file changed since it was loaded");
      })
      .catch(showError);
  };
  const importFile = () => {
    api
      .rulesImport()
      .then((imported) => {
        if (!imported) return;
        const backup = imported.backup ? `; the old file is ${imported.backup}` : "";
        useStore.getState().showToast("success", `Imported ${imported.fileName}${backup}`);
        void refetch();
        if (imported.needsReload) reload();
      })
      .catch(showError);
  };

  return (
    <>
      <Group title="Rulesets" anchor="rulesets">
        {files.map((file) => (
          <Row key={file.year} label={file.fileName} description={describeFile(file)}>
            <span className={PILLS[file.state].className}>{PILLS[file.state].label}</span>
            <button className="btn btn--small" disabled={file.missing && file.state !== "closed"} onClick={() => void api.rulesOpen(file.year).catch(showError)}>
              {file.state === "closed" ? "View" : "Open in Editor"}
            </button>
          </Row>
        ))}
      </Group>
      <Group title="Actions" anchor="rules-actions">
        <Row label="Reload after editing" description={reloadText(files)} anchor="reload">
          <button className="btn btn--small" onClick={reload}>
            Reload…
          </button>
        </Row>
        <Row label="Import a JSON file" description="Replaces the ruleset of a draft or open year. The old file stays in the folder as a backup." anchor="import">
          <button className="btn btn--small" onClick={importFile}>
            Import…
          </button>
        </Row>
        <Row
          label="Start next year"
          description={newestOpen === undefined ? "Needs an open tax year to copy from." : nextExists ? `rules-${newestOpen + 1}.json exists.` : `Copies rules-${newestOpen}.json to rules-${newestOpen + 1}.json as a draft.`}
          anchor="duplicate"
        >
          <button className="btn btn--small" disabled={newestOpen === undefined || nextExists} onClick={() => setDuplicating(true)}>
            Duplicate…
          </button>
        </Row>
        <Row label="Rules folder" description={<span className="mono">{folder ? `${folder}rules/` : ""}</span>} anchor="rules-folder">
          <button className="btn btn--small" onClick={() => void api.rulesShowFolder().catch(showError)}>
            Show in Finder
          </button>
        </Row>
      </Group>
      <Group title="Format" anchor="rules-format">
        <Row label="Excerpt of a rules file" description="rules.schema.json and README.md sit next to the files; the README explains every field." />
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
