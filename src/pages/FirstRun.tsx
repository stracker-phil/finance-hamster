import { useState } from "react";
import { AccountForm } from "../accounts/AccountForm";
import { useTaxYears } from "../queries/taxYears";
import { startImport } from "../statements/importFlow";
import { useT } from "../i18n";
import { ProfileSwitcher } from "../shell/ProfileSwitcher";

interface Props {
  onAccountAdded: () => void;
  onDone: () => void;
}

/** The first launch, without the sidebar: add the first account, then import the first statement or skip. The
 *  profile switcher stays reachable, so a new, empty profile can go back to another. */
export function FirstRun({ onAccountAdded, onDone }: Props) {
  const [step, setStep] = useState<"account" | "statement">("account");
  const { data: years = [] } = useTaxYears();
  const year = years.find((y) => y.state === "open");
  const text = useT().pages.firstRun;
  return (
    <div className="first-run">
      <div className="first-run__titlebar" data-tauri-drag-region>
        <ProfileSwitcher />
      </div>
      <div className="first-run__card">
        <div className="first-run__step">{text.step(step === "account" ? 1 : 2)}</div>
        {step === "account" ? (
          <>
            <h1>{text.accountTitle}</h1>
            <p>{text.accountText}</p>
            <AccountForm
              submitLabel={text.continue}
              onSaved={() => {
                onAccountAdded();
                setStep("statement");
              }}
            />
          </>
        ) : (
          <>
            <h1>{text.statementTitle}</h1>
            <p>{text.statementText}</p>
            {year && (
              <p>{text.ready(year.year)}</p>
            )}
            <div className="actions">
              <button type="button" className="btn" onClick={onDone}>
                {text.skip}
              </button>
              <button type="button" className="btn btn--primary" onClick={startImport}>
                {text.import}
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
