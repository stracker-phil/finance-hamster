import { useLanguage } from "../i18n";

const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const MONTHS_DE = ["Jan.", "Feb.", "März", "Apr.", "Mai", "Juni", "Juli", "Aug.", "Sept.", "Okt.", "Nov.", "Dez."];

/** `13 Nov 2026` / `13. Nov. 2026`, the way the Rust side writes dates (day first). */
export function dayMonthYear(day: Date): string {
  if (useLanguage.getState().language === "de") return `${day.getDate()}. ${MONTHS_DE[day.getMonth()]} ${day.getFullYear()}`;
  return `${day.getDate()} ${MONTHS[day.getMonth()]} ${day.getFullYear()}`;
}
