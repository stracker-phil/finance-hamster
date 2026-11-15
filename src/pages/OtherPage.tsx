import { useId, useState } from "react";
import { api, toAppError, type OtherEdit } from "../api";
import { SavedInput, useDraftText } from "../components/SavedInput";
import { DetailLayout } from "../documents/DetailLayout";
import { useT } from "../i18n";
import { showError, undoToast } from "../lib/undoToast";
import { useOther } from "../queries/others";
import { useStore } from "../state/store";
import { EmptyPage } from "./EmptyPage";

function NoteField({ value, label, onSave }: { value: string; label: string; onSave: (text: string) => void }) {
  const { text, setText, commit } = useDraftText(value, onSave);
  return <textarea className="field note-field" rows={3} aria-label={label} value={text} onChange={(e) => setText(e.target.value)} onBlur={commit} />;
}

/** Any other PDF: over most of the page, with its title, date and note in the sidebar. */
export function OtherPage({ otherId }: { otherId: string }) {
  const id = useId();
  const { data: doc, isError } = useOther(otherId);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const text = useT().payslips;
  const titles = useT().shell.titles;
  if (isError) return <EmptyPage title={text.otherMissingTitle} text={text.missingText} />;
  const save = (change: OtherEdit) => {
    api
      .otherEdit(otherId, change)
      .then((label) => {
        setErrors({});
        if (label) undoToast(label);
      })
      .catch((e: unknown) => {
        const failure = toAppError(e);
        if (failure.field) setErrors({ [failure.field.path]: failure.field.message });
        else showError(e);
      });
  };
  const remove = () => {
    api
      .otherDelete(otherId)
      .then((label) => {
        undoToast(label);
        useStore.getState().navigate({ page: "documents", tab: "other" });
      })
      .catch(showError);
  };
  return (
    <DetailLayout source={{ kind: "other", id: otherId }} label={doc?.title ?? titles.other}>
      {doc && (
        <div className="other-form">
          <label htmlFor={`${id}-title`}>{text.otherTitle}</label>
          <SavedInput id={`${id}-title`} value={doc.title} onSave={(value) => save({ field: "title", text: value })} />
          <div className="form-field__error" role={errors["other.title"] ? "alert" : undefined}>
            {errors["other.title"] ?? ""}
          </div>
          <label htmlFor={`${id}-date`}>{text.otherDate}</label>
          <SavedInput id={`${id}-date`} type="date" value={doc.date} onSave={(value) => save({ field: "date", value })} />
          <div className="form-field__error" role={errors["other.date"] ? "alert" : undefined}>
            {errors["other.date"] ?? ""}
          </div>
          <span>{text.note}</span>
          <NoteField value={doc.note} label={text.note} onSave={(value) => save({ field: "note", text: value })} />
          <div className="detail__actions">
            <button type="button" className="link-btn" onClick={remove}>
              {text.otherDelete}
            </button>
          </div>
        </div>
      )}
    </DetailLayout>
  );
}
