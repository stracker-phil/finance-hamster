import { Activity, Bell, Check, FilePlus, FileText, House, Info, Keyboard, Landmark, Store, Layers, ListChecks, Lock, Palette, PanelLeft, Repeat, Scale, Settings, ShieldCheck, type LucideIcon } from "lucide-react";

/** The app's icons by role, from lucide like the family's other apps (Taskboard); the mockup's drawn icons were
 *  placeholders. */
const ICONS = {
  home: House,
  repeat: Repeat,
  stack: Layers,
  docs: FileText,
  addDoc: FilePlus,
  sidebar: PanelLeft,
  settings: Settings,
  lock: Lock,
  privacy: ShieldCheck,
  reminders: Bell,
  appearance: Palette,
  shortcuts: Keyboard,
  status: Activity,
  about: Info,
  check: Check,
  accounts: Landmark,
  taxRules: Scale,
  rules: ListChecks,
  vendors: Store,
} satisfies Record<string, LucideIcon>;

export type IconName = keyof typeof ICONS;

export function Icon({ name, size = 17, strokeWidth = 1.8 }: { name: IconName; size?: number; strokeWidth?: number }) {
  const Glyph = ICONS[name];
  return <Glyph size={size} strokeWidth={strokeWidth} aria-hidden="true" />;
}
