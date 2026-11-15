import { shell } from "./messages/shell";
import { components } from "./messages/components";
import { review } from "./messages/review";
import { statements } from "./messages/statements";
import { pages } from "./messages/pages";
import { documents } from "./messages/documents";
import { subscriptions } from "./messages/subscriptions";
import { year } from "./messages/year";
import { settings } from "./messages/settings";

/** The English texts, one area per file in `messages/`; each holds both languages (ADR 0050). */
export const en = {
  shell: shell.en,
  components: components.en,
  review: review.en,
  statements: statements.en,
  pages: pages.en,
  documents: documents.en,
  subscriptions: subscriptions.en,
  year: year.en,
  settings: settings.en,
};

export type Messages = typeof en;
