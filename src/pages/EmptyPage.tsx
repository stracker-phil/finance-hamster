import type { ReactNode } from "react";

/** A page that has nothing to show yet; `children` is its one action. */
export function EmptyPage({ title, text, children }: { title: string; text: string; children?: ReactNode }) {
  return (
    <div className="empty-page">
      <div className="empty-page__title">{title}</div>
      <div>{text}</div>
      {children}
    </div>
  );
}
