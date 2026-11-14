const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/** `13 Nov 2026`, the way the Rust side writes dates (day first). */
export const dayMonthYear = (day: Date) => `${day.getDate()} ${MONTHS[day.getMonth()]} ${day.getFullYear()}`;
