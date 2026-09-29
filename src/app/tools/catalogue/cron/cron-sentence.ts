import { CronField, FieldReading, Piece } from '@core/model/tool-answers.model';

export type Translate = (key: string, params?: Record<string, unknown>) => string;
export type JoinList = (items: readonly string[]) => string;

type Of<K extends Piece['kind']> = Extract<Piece, { kind: K }>;

const SAY = 'tools.cron.say';

function isEvery(pieces: readonly Piece[] | undefined): boolean {
  return pieces === undefined || pieces.every((piece) => piece.kind === 'every');
}

/** The values of a field made only of values, or `null`. */
function valuesOf(pieces: readonly Piece[] | undefined): number[] | null {
  if (pieces === undefined || pieces.length === 0) return null;
  const values = pieces.map((piece) => (piece.kind === 'value' ? piece.value : null));
  return values.every((value) => value !== null) ? (values as number[]) : null;
}

const two = (value: number): string => String(value).padStart(2, '0');

/**
 * A cron expression read aloud, from the fields Rust took apart: the words are translation keys,
 * the grammar the smallest that reads well — times written as times when the minute and the hour
 * are plain values, otherwise one clause per field.
 */
export function cronSentence(fields: readonly FieldReading[], t: Translate, join: JoinList): string {
  const byField = new Map<CronField, readonly Piece[]>(fields.map((field) => [field.field, field.pieces]));
  const format = (field: CronField, value: number): string => {
    switch (field) {
      case 'hour':
        return t(`${SAY}.hourValue`, { hour: value });
      case 'dayOfMonth':
        return t(`${SAY}.dayValue`, { day: value });
      case 'month':
        return t(`tools.cron.months.${value}`);
      case 'dayOfWeek':
        return t(`tools.cron.days.${value % 7}`);
      default:
        return String(value);
    }
  };

  const clause = (field: CronField, pieces: readonly Piece[]): string => {
    const values = pieces
      .filter((piece): piece is Of<'value'> => piece.kind === 'value')
      .map((piece) => piece.value);
    const phrases: string[] = [];
    if (values.length > 0) {
      phrases.push(
        t(`${SAY}.${field}.values`, {
          count: values.length,
          list: join(values.map((value) => format(field, value))),
        }),
      );
    }
    for (const piece of pieces) {
      phrases.push(...piecePhrase(field, piece, format, t));
    }
    return join(phrases);
  };

  const clauses: string[] = [];
  const seconds = byField.get('second');
  const minutes = byField.get('minute');
  const hours = byField.get('hour');
  const plainSeconds =
    seconds === undefined || (valuesOf(seconds)?.length === 1 && valuesOf(seconds)?.[0] === 0);
  const minuteValues = valuesOf(minutes);
  const hourValues = valuesOf(hours);

  if (!plainSeconds && seconds) {
    clauses.push(isEvery(seconds) ? t(`${SAY}.second.every`) : clause('second', seconds));
  }
  if (plainSeconds && minuteValues && hourValues && minuteValues.length * hourValues.length <= 8) {
    const times = hourValues.flatMap((hour) => minuteValues.map((minute) => `${two(hour)}:${two(minute)}`));
    clauses.push(t(`${SAY}.at`, { count: times.length, list: join(times.sort()) }));
  } else if (minuteValues?.length === 1 && minuteValues[0] === 0 && hourValues === null) {
    clauses.push(isEvery(hours) ? t(`${SAY}.hour.every`) : clause('hour', hours ?? []));
  } else {
    if (isEvery(minutes)) {
      if (plainSeconds) clauses.push(t(`${SAY}.minute.every`));
    } else {
      clauses.push(clause('minute', minutes ?? []));
    }
    if (!isEvery(hours)) clauses.push(clause('hour', hours ?? []));
  }

  for (const field of ['dayOfMonth', 'dayOfWeek', 'month'] as const) {
    const pieces = byField.get(field);
    if (!isEvery(pieces)) clauses.push(clause(field, pieces ?? []));
  }

  const sentence = clauses.join(', ');
  return sentence.charAt(0).toUpperCase() + sentence.slice(1);
}

function piecePhrase(
  field: CronField,
  piece: Piece,
  format: (field: CronField, value: number) => string,
  t: Translate,
): string[] {
  switch (piece.kind) {
    case 'range':
      return [t(`${SAY}.${field}.range`, { from: format(field, piece.from), to: format(field, piece.to) })];
    case 'step':
      return [t(`${SAY}.${field}.step`, { step: piece.step })];
    case 'steppedRange':
      return [
        t(`${SAY}.${field}.steppedRange`, {
          step: piece.step,
          from: format(field, piece.from),
          to: format(field, piece.to),
        }),
      ];
    case 'lastDayOfMonth':
      return [t(`${SAY}.dayOfMonth.last`)];
    case 'nearestWeekday':
      return [t(`${SAY}.dayOfMonth.nearest`, { day: format('dayOfMonth', piece.day) })];
    case 'lastWeekday':
      return [t(`${SAY}.dayOfWeek.last`, { day: format('dayOfWeek', piece.weekday) })];
    case 'nth':
      return [
        t(`${SAY}.dayOfWeek.nth`, {
          nth: t(`tools.cron.nths.${piece.nth}`),
          day: format('dayOfWeek', piece.weekday),
        }),
      ];
    default:
      return [];
  }
}
