/** A page that has nothing to show yet. */
export function EmptyPage({ title, text }: { title: string; text: string }) {
  return (
    <div className="empty-page">
      <div className="empty-page__title">{title}</div>
      <div>{text}</div>
    </div>
  );
}
