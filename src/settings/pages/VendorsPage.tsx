import { useId, useState } from "react";
import { api, toAppError, type VendorChange, type VendorView } from "../../api";
import { PopUpButton } from "../../components/menu";
import { Modal } from "../../components/Modal";
import { t, useT } from "../../i18n";
import { useVendors } from "../../queries/documents";
import { useStore } from "../../state/store";
import { Group, Row } from "../controls";

interface VendorSheetProps {
  vendor: VendorView;
  /** The vendors it can merge into. */
  others: VendorView[];
  items: { key: string; label: string }[];
  onClose: () => void;
}

/** The changes that turn `vendor` into the typed name and chosen default item. */
function vendorChanges(vendor: VendorView, name: string, item: string): VendorChange[] {
  const changes: VendorChange[] = [];
  if (name.trim() !== vendor.name) changes.push({ kind: "rename", name: name.trim() });
  if (item !== (vendor.defaultItemKey ?? "")) changes.push({ kind: "defaultItem", key: item || null });
  return changes;
}

/** Renames a vendor, sets its default item, merges it into another or deletes it when unused. */
function VendorSheet({ vendor, others, items, onClose }: VendorSheetProps) {
  const id = useId();
  const { vendors: text, common } = useT().settings;
  const [name, setName] = useState(vendor.name);
  const [item, setItem] = useState(vendor.defaultItemKey ?? "");
  const [into, setInto] = useState("");
  const [error, setError] = useState<string | null>(null);
  const run = async (changes: VendorChange[]) => {
    try {
      const labels: string[] = [];
      for (const change of changes) labels.push(await api.vendorChange(vendor.id, change));
      useStore.getState().showToast("success", labels.join(" · "));
      onClose();
    } catch (e) {
      const failure = toAppError(e);
      setError(failure.field?.message ?? failure.message);
    }
  };
  const save = () => {
    const changes = vendorChanges(vendor, name, item);
    if (changes.length === 0) onClose();
    else void run(changes);
  };
  const merge = async () => {
    const target = others.find((o) => o.id === into);
    if (!target) return;
    const words = t().settings.vendors;
    const confirmed = await useStore.getState().confirm({ title: words.mergeTitle(vendor.name, target.name), text: words.mergeText(vendor.name, target.name), action: words.mergeAction });
    if (confirmed) void run([{ kind: "mergeInto", into }]);
  };
  return (
    <Modal labelledBy={id} className="sheet--narrow" onClose={onClose}>
      <h2 id={id}>{text.sheetTitle}</h2>
      <div className="rule-form">
        <div className="form-field">
          <label htmlFor={`${id}-name`}>{text.name}</label>
          <input id={`${id}-name`} className="field" value={name} autoFocus onChange={(e) => setName(e.target.value)} />
          <span className="form-check__desc">{text.nameHelp}</span>
        </div>
        <div className="form-field">
          <label htmlFor={`${id}-item`}>{text.itemLabel}</label>
          <PopUpButton
            id={`${id}-item`}
            value={item}
            options={[{ value: "", label: text.lastItem }, ...items.map((i) => ({ value: i.key, label: i.label }))]}
            onChange={setItem}
          />
        </div>
        {others.length > 0 && (
          <div className="form-field">
            <label htmlFor={`${id}-merge`}>{text.mergeInto}</label>
            <div className="add-doc__field">
              <PopUpButton
                id={`${id}-merge`}
                value={into}
                options={[{ value: "", label: text.chooseVendor }, ...others.map((o) => ({ value: o.id, label: o.name }))]}
                onChange={setInto}
              />
              <button type="button" className="btn" disabled={!into} onClick={() => void merge()}>
                {text.mergeButton}
              </button>
            </div>
          </div>
        )}
        <div className="form-field__error" role={error ? "alert" : undefined}>
          {error ?? ""}
        </div>
        <div className="actions">
          {vendor.unused && (
            <button type="button" className="btn btn--danger-text rule-form__delete" onClick={() => void run([{ kind: "delete" }])}>
              {common.delete}
            </button>
          )}
          <button type="button" className="btn" onClick={onClose}>
            {common.cancel}
          </button>
          <button type="button" className="btn btn--primary" onClick={save}>
            {common.save}
          </button>
        </div>
      </div>
    </Modal>
  );
}

/** Settings → Vendors: every vendor with its documents and item. */
export function VendorsPage() {
  const { data } = useVendors();
  const [editing, setEditing] = useState<VendorView | null>(null);
  const { vendors: text, common } = useT().settings;
  return (
    <>
      <Group title={text.list} anchor="vendor-list">
        {data?.vendors.length === 0 && <Row label={text.none} description={text.noneDescription} />}
        {data?.vendors.map((v) => (
          <Row key={v.id} label={v.name} description={v.detail}>
            <button className="btn btn--small" onClick={() => setEditing(v)}>
              {common.edit}
            </button>
          </Row>
        ))}
      </Group>
      <Group title={text.rules} anchor="vendor-rules">
        <Row label={text.defaultItem} description={text.defaultItemDescription} />
        <Row label={text.deleteVendor} description={text.deleteVendorDescription} />
      </Group>
      {editing && data && <VendorSheet vendor={editing} others={data.vendors.filter((v) => v.id !== editing.id)} items={data.items} onClose={() => setEditing(null)} />}
    </>
  );
}
