import { useId } from "react";
import { api } from "../api";
import { Modal } from "../components/Modal";
import { useT } from "../i18n";
import { showError, undoToast } from "../lib/undoToast";
import { useYearCategories } from "../queries/year";
import { Group, Switch } from "../settings/controls";

const setShown = (year: number, key: string, shown: boolean) => {
  api
    .yearSetCategoryShown(year, key, shown)
    .then((label) => label && undoToast(label))
    .catch(showError);
};

/** One switch per category of `year`: whether it shows while recording. */
export function CategorySwitches({ year }: { year: number }) {
  const { data = [] } = useYearCategories(year);
  const text = useT().year.categories;
  return (
    <Group title={text.heading}>
      {data.map((c) => (
        <Switch key={c.key} label={c.label} description={c.examples ?? undefined} checked={c.shown} onChange={(on) => setShown(year, c.key, on)} />
      ))}
    </Group>
  );
}

/** The switches as a sheet, from the tax year's … menu. */
export function CategoriesSheet({ year, onClose }: { year: number; onClose: () => void }) {
  const id = useId();
  const text = useT().year.categories;
  return (
    <Modal labelledBy={id} className="sheet categories-sheet" onClose={onClose}>
      <h2 id={id}>{text.title(year)}</h2>
      <p>{text.intro}</p>
      <div className="categories-sheet__body">
        <CategorySwitches year={year} />
      </div>
      <div className="actions">
        <button type="button" className="btn btn--primary" onClick={onClose}>
          {text.done}
        </button>
      </div>
    </Modal>
  );
}
