/** Between thousands: a narrow no-break space in French, a comma in English. */
const GROUPS: Record<string, string> = { fr: ' ', en: ',' };
const DECIMAL_MARKS: Record<string, string> = { fr: ',', en: '.' };

/**
 * An exact decimal as Rust wrote it (`-1234567.891`), written for the language on screen
 * (`-1 234 567,891`). Text in, text out: a float would round what Rust kept exact.
 */
export function formatDecimal(value: string, lang: string): string {
  const match = /^(-?)(\d+)(?:\.(\d+))?$/.exec(value);
  if (!match) return value;
  const [, sign, whole, fraction] = match;
  const grouped = whole!.replace(/\B(?=(\d{3})+(?!\d))/g, GROUPS[lang] ?? ',');
  return fraction === undefined
    ? `${sign}${grouped}`
    : `${sign}${grouped}${DECIMAL_MARKS[lang] ?? '.'}${fraction}`;
}
