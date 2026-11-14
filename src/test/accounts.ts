import type { Account } from "../api";

/** A demo bank account, with `patch` applied. */
export function account(patch: Partial<Account> = {}): Account {
  return { id: "acc-1", name: "Private", bank: "Example Bank", ibanMasked: "AT•• •••• 0001", own: true, statementName: null, ...patch };
}
