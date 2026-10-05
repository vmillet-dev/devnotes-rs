import { SizeUnit } from '@core/model/tool-answers.model';
import { ChoiceOption } from '@shared/controls/choice-menu/choice-menu.component';

export const SIZE_GROUPS: readonly { readonly id: string; readonly units: readonly SizeUnit[] }[] = [
  { id: 'bytes', units: ['byte', 'bit', 'kilobit', 'megabit', 'gigabit', 'terabit'] },
  { id: 'decimal', units: ['kilobyte', 'megabyte', 'gigabyte', 'terabyte', 'petabyte'] },
  { id: 'binary', units: ['kibibyte', 'mebibyte', 'gibibyte', 'tebibyte', 'pebibyte'] },
];

export const SIZE_UNITS: readonly ChoiceOption[] = SIZE_GROUPS.flatMap((group) =>
  group.units.map((unit) => ({ id: unit, name: `tools.sizes.unitNames.${unit}`, nameIsKey: true })),
);
