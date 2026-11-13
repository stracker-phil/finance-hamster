import { createContext, useContext, useEffect, useId, useRef, type ReactNode } from "react";

/** The row Settings opens at; a new object per request, so asking again reveals it again. */
export const RevealTarget = createContext<{ id: string } | undefined>(undefined);

/** Scrolls to and washes the element whose `anchor` the open request names. */
function useReveal<E extends HTMLElement>(anchor: string | undefined) {
  const target = useContext(RevealTarget);
  const ref = useRef<E>(null);
  useEffect(() => {
    const element = ref.current;
    if (!anchor || target?.id !== anchor || !element) return;
    element.scrollIntoView?.({ block: "start" });
    element.classList.remove("flash");
    void element.offsetWidth;
    element.classList.add("flash");
  }, [anchor, target]);
  return ref;
}

/** An inset group with its heading above it. */
export function Group({ title, anchor, children }: { title?: string; anchor?: string; children: ReactNode }) {
  const ref = useReveal<HTMLElement>(anchor);
  return (
    <section ref={ref} className="settings-group" aria-label={title}>
      {title && <h3>{title}</h3>}
      <div className="settings-card">{children}</div>
    </section>
  );
}

interface RowProps {
  label: ReactNode;
  description?: ReactNode;
  anchor?: string;
  htmlFor?: string;
  children?: ReactNode;
}

/** One row: label and description on the left, the control (a value, a button, a switch) on the right. */
export function Row({ label, description, anchor, htmlFor, children }: RowProps) {
  const ref = useReveal<HTMLDivElement>(anchor);
  return (
    <div ref={ref} className="settings-row">
      <div className="settings-row__text">
        {htmlFor ? <label htmlFor={htmlFor}>{label}</label> : <div>{label}</div>}
        {description && <div className="settings-row__desc">{description}</div>}
      </div>
      {children !== undefined && <div className="settings-row__control">{children}</div>}
    </div>
  );
}

/** A row whose control is a switch; its state shows by color only, so it keeps its size. */
export function Switch({ label, description, anchor, checked, onChange }: { label: string; description?: ReactNode; anchor?: string; checked: boolean; onChange: (on: boolean) => void }) {
  const id = useId();
  return (
    <Row label={label} description={description} anchor={anchor} htmlFor={id}>
      <input id={id} className="switch" type="checkbox" role="switch" checked={checked} onChange={(e) => onChange(e.target.checked)} />
    </Row>
  );
}

/** Mutually exclusive buttons; the pressed one is raised. */
export function Segmented<T extends string>({ label, options, value, onChange }: { label: string; options: { value: T; label: string }[]; value: T | undefined; onChange: (v: T) => void }) {
  return (
    <div className="seg" role="group" aria-label={label}>
      {options.map((o) => (
        <button key={o.value} type="button" className="seg-btn" aria-pressed={value === o.value} onClick={() => onChange(o.value)}>
          {o.label}
        </button>
      ))}
    </div>
  );
}
