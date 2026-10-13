import { LANGUAGE_LABELS, LANGUAGE_NAMES, LanguageTag } from '@core/model/language.model';
import type { ChoiceOption } from '@shared/controls/choice-menu/choice-menu.component';

/** The formats as the note's menu and the code block's menu both offer them. */
export const LANGUAGE_CHOICES: readonly ChoiceOption[] = (
  Object.entries(LANGUAGE_LABELS) as [LanguageTag, string][]
).map(([id, name]) => {
  const hint = LANGUAGE_NAMES[id];
  return hint === undefined ? { id, name } : { id, name, hint };
});
