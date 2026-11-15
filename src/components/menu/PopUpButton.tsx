import { ChevronsUpDown } from "lucide-react";
import { useState, type Ref } from "react";
import { useStore } from "../../state/store";

export interface PopUpOption<T extends string | number> {
  value: T;
  label: string;
}

interface Props<T extends string | number> {
  value: T;
  options: readonly PopUpOption<T>[];
  onChange: (value: T) => void;
  /** For a `<label htmlFor>` naming it. */
  id?: string;
  "aria-label"?: string;
  "aria-invalid"?: boolean;
  disabled?: boolean;
  className?: string;
  ref?: Ref<HTMLButtonElement>;
}

/** A macOS pop-up button in place of a `<select>`: the current choice and a chevron, as wide as its widest
 *  choice, so choosing never resizes it. */
export function PopUpButton<T extends string | number>({ value, options, onChange, id, "aria-label": label, "aria-invalid": invalid, disabled, className, ref }: Props<T>) {
  const [button, setButton] = useState<HTMLButtonElement | null>(null);
  const open = useStore((s) => button !== null && s.menu?.at === button);
  const current = options.find((o) => o.value === value);

  const show = () => {
    if (!button) return;
    const entries = options.map((o) => ({
      text: o.label,
      checked: o.value === value,
      run: () => {
        if (o.value !== value) onChange(o.value);
      },
    }));
    useStore.getState().openMenu(entries, button, label ?? button.labels?.[0]?.textContent ?? undefined);
  };

  return (
    <button
      ref={(element) => {
        setButton(element);
        if (typeof ref === "function") ref(element);
        else if (ref) ref.current = element;
      }}
      type="button"
      id={id}
      className={className ? `field popup-button ${className}` : "field popup-button"}
      aria-label={label}
      aria-haspopup="menu"
      aria-expanded={open}
      aria-invalid={invalid || undefined}
      disabled={disabled}
      onClick={show}
      onKeyDown={(e) => {
        if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
        e.preventDefault();
        if (!open) show();
      }}
    >
      <span className="popup-button__value">
        <span>{current?.label ?? ""}</span>
        {options.map((o) => (
          <span key={String(o.value)} className="popup-button__sizer" aria-hidden="true">
            {o.label}
          </span>
        ))}
      </span>
      <ChevronsUpDown className="popup-button__chevron" size={12} strokeWidth={2.2} aria-hidden="true" />
    </button>
  );
}
