/** The number of `acct-N` color classes in statements.css; an account's place in the list picks its class. */
const ACCOUNT_TONES = 6;

export const accountTone = (index: number) => `acct-${index % ACCOUNT_TONES}`;
