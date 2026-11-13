/** The mockup's line icons (24px grid), for the places lucide has no match for its look. */
const PATHS = {
  home: "M4 11l8-7 8 7v9h-5v-6H9v6H4z",
  repeat: "M17 2l3 3-3 3M3 11V9a4 4 0 0 1 4-4h13M7 22l-3-3 3-3M21 13v2a4 4 0 0 1-4 4H4",
  stack: "M4 7l8-4 8 4-8 4zM4 12l8 4 8-4M4 17l8 4 8-4",
  docs: "M14 3H6v18h12V7zM14 3v4h4M9 13h6M9 17h4",
  addDoc: "M14 3H6v18h12V7zM14 3v4h4M12 11v6M9 14h6",
  sidebar: "M4 5h16v14H4zM9 5v14",
  settings: "M12 9a3 3 0 1 0 0 6 3 3 0 0 0 0-6zM12 2.5v3M12 18.5v3M2.5 12h3M18.5 12h3M5.3 5.3l2.1 2.1M16.6 16.6l2.1 2.1M5.3 18.7l2.1-2.1M16.6 7.4l2.1-2.1",
  lock: "M6 11h12v10H6zM8 11V7a4 4 0 0 1 8 0v4",
} as const;

export type IconName = keyof typeof PATHS;

export function Icon({ name, size = 17, strokeWidth = 1.8 }: { name: IconName; size?: number; strokeWidth?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={strokeWidth} strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d={PATHS[name]} />
    </svg>
  );
}
