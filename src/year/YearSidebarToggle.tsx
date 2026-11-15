import { PanelRight } from "lucide-react";
import { useT } from "../i18n";
import { togglePayslipPanel } from "../payslips/flow";
import { useSettings } from "../queries/settings";

/** At the toolbar's trailing edge on the tax year page: shows or hides the year's sidebar, like a macOS inspector. */
export function YearSidebarToggle() {
  const open = useSettings().data?.payslipPanel ?? false;
  const text = useT().payslips;
  return (
    <button type="button" className="icon-btn header__toggle" aria-label={text.toggle} aria-pressed={open} title={text.toggle} onClick={togglePayslipPanel}>
      <PanelRight size={17} />
    </button>
  );
}
