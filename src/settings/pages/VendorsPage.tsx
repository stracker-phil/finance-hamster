import { useId, useState } from "react";
import { api, toAppError, type VendorChange, type VendorView } from "../../api";
import { Modal } from "../../components/Modal";
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
    const confirmed = await useStore.getState().confirm({ title: `Merge ${vendor.name} into ${target.name}?`, text: `Its documents move to ${target.name} and ${vendor.name} is removed. You can undo it.`, action: "Merge" });
    if (confirmed) void run([{ kind: "mergeInto", into }]);
  };
  return (
    <Modal labelledBy={id} className="sheet--narrow" onClose={onClose}>
      <h2 id={id}>Edit Vendor</h2>
      <div className="rule-form">
        <div className="form-field">
          <label htmlFor={`${id}-name`}>Name</label>
          <input id={`${id}-name`} className="field" value={name} autoFocus onChange={(e) => setName(e.target.value)} />
          <span className="form-check__desc">Renaming moves the files of drafts; finished documents keep their names.</span>
        </div>
        <div className="form-field">
          <label htmlFor={`${id}-item`}>Default item</label>
          <select id={`${id}-item`} className="field" value={item} onChange={(e) => setItem(e.target.value)}>
            <option value="">The item of its last document</option>
            {items.map((i) => (
              <option key={i.key} value={i.key}>
                {i.label}
              </option>
            ))}
          </select>
        </div>
        {others.length > 0 && (
          <div className="form-field">
            <label htmlFor={`${id}-merge`}>Merge into</label>
            <div className="add-doc__field">
              <select id={`${id}-merge`} className="field" value={into} onChange={(e) => setInto(e.target.value)}>
                <option value="">Choose a vendor</option>
                {others.map((o) => (
                  <option key={o.id} value={o.id}>
                    {o.name}
                  </option>
                ))}
              </select>
              <button type="button" className="btn" disabled={!into} onClick={() => void merge()}>
                Merge…
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
              Delete
            </button>
          )}
          <button type="button" className="btn" onClick={onClose}>
            Cancel
          </button>
          <button type="button" className="btn btn--primary" onClick={save}>
            Save
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
  return (
    <>
      <Group title="Vendors" anchor="vendor-list">
        {data?.vendors.length === 0 && <Row label="No vendors yet" description="Typing a new vendor on a document adds it here." />}
        {data?.vendors.map((v) => (
          <Row key={v.id} label={v.name} description={v.detail}>
            <button className="btn btn--small" onClick={() => setEditing(v)}>
              Edit…
            </button>
          </Row>
        ))}
      </Group>
      <Group title="Rules for vendors" anchor="vendor-rules">
        <Row label="Default item per vendor" description="A new document from a vendor starts with its default item, or the item of its last document." />
        <Row label="Delete a vendor" description="Only possible when no document uses it; merge it into another vendor otherwise." />
      </Group>
      {editing && data && <VendorSheet vendor={editing} others={data.vendors.filter((v) => v.id !== editing.id)} items={data.items} onClose={() => setEditing(null)} />}
    </>
  );
}
