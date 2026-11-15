import { useEffect, useState } from "react";

/** Keeps typing local so each keystroke doesn't save; follows the stored value when it changes. */
export function useDraftText(value: string, onSave: (text: string) => void) {
  const [text, setText] = useState(value);
  useEffect(() => setText(value), [value]);
  const commit = () => {
    if (text !== value) onSave(text);
  };
  return { text, setText, commit };
}

/** A text field that saves on leaving it or on Return. */
export function SavedInput({ id, value, onSave, className, ...props }: { id: string; value: string; onSave: (text: string) => void } & Omit<React.InputHTMLAttributes<HTMLInputElement>, "value" | "onChange">) {
  const { text, setText, commit } = useDraftText(value, onSave);
  return (
    <input
      id={id}
      className={className ? `field ${className}` : "field"}
      value={text}
      onChange={(e) => setText(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => {
        if (e.key === "Enter") e.currentTarget.blur();
      }}
      {...props}
    />
  );
}
