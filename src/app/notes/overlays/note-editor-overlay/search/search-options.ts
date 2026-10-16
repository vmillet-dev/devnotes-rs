import { PreferencesService } from '@core/services/preferences/preferences.service';

export type SearchOption = 'caseSensitive' | 'wholeWord' | 'regexp';

export type SearchOptions = Readonly<Record<SearchOption, boolean>>;

const OPTIONS_KEY = 'devnotes.editorSearchOptions';

const NO_OPTIONS: SearchOptions = { caseSensitive: false, wholeWord: false, regexp: false };

/** For the application, like full screen: a regex turned on stays on from note to note, and editor to editor. */
export function readSearchOptions(preferences: PreferencesService): SearchOptions {
  try {
    const stored: unknown = JSON.parse(preferences.read(OPTIONS_KEY) ?? '{}');
    const value = (stored ?? {}) as Partial<Record<SearchOption, unknown>>;
    return {
      caseSensitive: value.caseSensitive === true,
      wholeWord: value.wholeWord === true,
      regexp: value.regexp === true,
    };
  } catch {
    return NO_OPTIONS;
  }
}

export function writeSearchOptions(preferences: PreferencesService, options: SearchOptions): void {
  preferences.write(OPTIONS_KEY, JSON.stringify(options));
}
