import type { Messages } from "./en";
import { shell } from "./messages/shell";
import { components } from "./messages/components";
import { review } from "./messages/review";
import { statements } from "./messages/statements";
import { pages } from "./messages/pages";
import { documents } from "./messages/documents";
import { subscriptions } from "./messages/subscriptions";
import { year } from "./messages/year";
import { settings } from "./messages/settings";
import { payslips } from "./messages/payslips";

/** Die deutschen Texte, Du-Form, de-DE (ADR 0050). */
export const de: Messages = {
  shell: shell.de,
  components: components.de,
  review: review.de,
  statements: statements.de,
  pages: pages.de,
  documents: documents.de,
  subscriptions: subscriptions.de,
  year: year.de,
  settings: settings.de,
  payslips: payslips.de,
};
