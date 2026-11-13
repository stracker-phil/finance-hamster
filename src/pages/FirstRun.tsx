import { useState } from "react";
import { AccountForm } from "../accounts/AccountForm";
import { useTaxYears } from "../queries/taxYears";
import { startImport } from "../statements/importFlow";

interface Props {
  onAccountAdded: () => void;
  onDone: () => void;
}

/** The first launch, without the sidebar: add the first account, then import the first statement or skip. */
export function FirstRun({ onAccountAdded, onDone }: Props) {
  const [step, setStep] = useState<"account" | "statement">("account");
  const { data: years = [] } = useTaxYears();
  const year = years.find((y) => y.state === "open");
  return (
    <div className="first-run">
      <div className="first-run__titlebar" data-tauri-drag-region />
      <div className="first-run__card">
        <div className="first-run__step">Step {step === "account" ? 1 : 2} of 2</div>
        {step === "account" ? (
          <>
            <h1>Add your first account</h1>
            <p>The bank account whose statements you import. Hamster keeps every statement and asks only about lines it can&apos;t sort itself.</p>
            <AccountForm
              submitLabel="Continue"
              onSaved={() => {
                onAccountAdded();
                setStep("statement");
              }}
            />
          </>
        ) : (
          <>
            <h1>Import your first statement</h1>
            <p>Choose a text PDF statement from your bank. You can also do this later from + New.</p>
            {year && (
              <p>
                Tax year {year.year} is ready with the rules {year.ruleset}.
              </p>
            )}
            <div className="actions">
              <button type="button" className="btn" onClick={onDone}>
                Skip for Now
              </button>
              <button type="button" className="btn btn--primary" onClick={startImport}>
                Import Statement…
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
