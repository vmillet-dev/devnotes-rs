/** Code that produces user-facing text returns one of these, never a formatted string. */
export interface TranslationRef {
  readonly key: string;
  readonly params?: Record<string, unknown>;
}
