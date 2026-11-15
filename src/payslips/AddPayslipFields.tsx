import { useEffect, useId, useState } from "react";
import { api } from "../api";
import { PopUpButton } from "../components/menu";
import { useT } from "../i18n";
import { showError } from "../lib/undoToast";
import { usePayslipsYear } from "../queries/payslips";
import { useTaxYears } from "../queries/taxYears";
import { addWaitingPayslip } from "./flow";

/** The Add sheet's payslip side: which open tax year and month the dropped PDF is for, guessed from its top. */
export function AddPayslipFields({ token, onCancel }: { token: string; onCancel: () => void }) {
  const id = useId();
  const text = useT().payslips;
  const cancel = useT().documents.add.cancel;
  const open = (useTaxYears().data ?? []).filter((y) => y.state === "open").map((y) => y.year);
  const [year, setYear] = useState<number | null>(null);
  const [month, setMonth] = useState<number>(new Date().getMonth() + 1);
  const [guessed, setGuessed] = useState(false);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    api
      .payslipGuess(token)
      .then((guess) => {
        if (!guess) return;
        setYear(guess[0]);
        setMonth(guess[1]);
        setGuessed(true);
      })
      .catch(() => undefined);
  }, [token]);
  // Tax years come newest first.
  const chosenYear = year ?? open[0] ?? new Date().getFullYear();
  const { data } = usePayslipsYear(chosenYear, open.includes(chosenYear));
  const taken = data?.months.find((m) => m.month === month)?.id != null;
  const add = () => {
    setBusy(true);
    addWaitingPayslip(token, chosenYear, month).catch((e: unknown) => {
      showError(e);
      setBusy(false);
    });
  };
  return (
    <>
      <div className="add-doc__grid">
        <label htmlFor={`${id}-year`}>{text.taxYear}</label>
        <PopUpButton id={`${id}-year`} value={chosenYear} options={open.map((y) => ({ value: y, label: String(y) }))} onChange={setYear} />
        <label htmlFor={`${id}-month`}>{text.month}</label>
        <div className="add-doc__field">
          <PopUpButton
            id={`${id}-month`}
            value={month}
            options={(data?.months ?? []).map((m) => ({ value: m.month, label: m.id ? `${m.name} · ${text.taken}` : m.name }))}
            onChange={setMonth}
          />
          {guessed && <span className="add-doc__hint">{text.fromPdf}</span>}
        </div>
      </div>
      <div className="actions">
        <button type="button" className="btn" onClick={onCancel}>
          {cancel}
        </button>
        <button type="button" className="btn btn--primary" disabled={busy || taken || !open.includes(chosenYear)} onClick={add}>
          {text.addPayslip}
        </button>
      </div>
    </>
  );
}
