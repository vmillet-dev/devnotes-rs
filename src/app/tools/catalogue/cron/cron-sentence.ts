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
  const values = pieces.flatMap((piece) => (piece.kind === 'value' ? [piece.value] : []));
  return values.length === pieces.length ? values : null;
}

const two = (value: number): string => String(value).padStart(2, '0');

/** How the sentence is spoken: the translation, and the language's way of joining a list. */
interface Speech {
  readonly t: Translate;
  readonly join: JoinList;
}

function format(speech: Speech, field: CronField, value: number): string {
  switch (field) {
    case 'hour':
      return speech.t(`${SAY}.hourValue`, { hour: value });
    case 'dayOfMonth':
      return speech.t(`${SAY}.dayValue`, { day: value });
    case 'month':
      return speech.t(`tools.cron.months.${value}`);
    case 'dayOfWeek':
      return speech.t(`tools.cron.days.${value % 7}`);
    case 'second':
    case 'minute':
      return String(value);
  }
}

/** One field's clause: its plain values together, then each range, step or special day. */
function clause(speech: Speech, field: CronField, pieces: readonly Piece[]): string {
  const values = pieces
    .filter((piece): piece is Of<'value'> => piece.kind === 'value')
    .map((piece) => piece.value);
  const phrases: string[] = [];
  if (values.length > 0) {
    phrases.push(
      speech.t(`${SAY}.${field}.values`, {
        count: values.length,
        list: speech.join(values.map((value) => format(speech, field, value))),
      }),
    );
  }
  for (const piece of pieces) {
    phrases.push(...piecePhrase(speech, field, piece));
  }
  return speech.join(phrases);
}

/** Seconds, minutes and hours: as times when they are plain values, one clause each otherwise. */
function timeClauses(speech: Speech, byField: ReadonlyMap<CronField, readonly Piece[]>): string[] {
  const [seconds, minutes, hours] = [byField.get('second'), byField.get('minute'), byField.get('hour')];
  const secondValues = valuesOf(seconds);
  const plainSeconds = seconds === undefined || (secondValues?.length === 1 && secondValues[0] === 0);
  const minuteValues = valuesOf(minutes);
  const hourValues = valuesOf(hours);
  const clauses: string[] = [];

  if (!plainSeconds) {
    clauses.push(
      isEvery(seconds) ? speech.t(`${SAY}.second.every`) : clause(speech, 'second', seconds ?? []),
    );
  }
  if (plainSeconds && minuteValues && hourValues && minuteValues.length * hourValues.length <= 8) {
    const times = hourValues.flatMap((hour) => minuteValues.map((minute) => `${two(hour)}:${two(minute)}`));
    times.sort((a, b) => a.localeCompare(b));
    return [speech.t(`${SAY}.at`, { count: times.length, list: speech.join(times) })];
  }
  if (minuteValues?.length === 1 && minuteValues[0] === 0 && hourValues === null) {
    return [isEvery(hours) ? speech.t(`${SAY}.hour.every`) : clause(speech, 'hour', hours ?? [])];
  }
  if (!isEvery(minutes)) {
    clauses.push(clause(speech, 'minute', minutes ?? []));
  } else if (plainSeconds) {
    clauses.push(speech.t(`${SAY}.minute.every`));
  }
  if (!isEvery(hours)) clauses.push(clause(speech, 'hour', hours ?? []));
  return clauses;
}

/**
 * A cron expression read aloud, from the fields Rust took apart: the words are translation keys,
 * the grammar the smallest that reads well — times written as times when the minute and the hour
 * are plain values, otherwise one clause per field.
 */
export function cronSentence(fields: readonly FieldReading[], t: Translate, join: JoinList): string {
  const speech: Speech = { t, join };
  const byField = new Map<CronField, readonly Piece[]>(fields.map((field) => [field.field, field.pieces]));
  const clauses = timeClauses(speech, byField);
  for (const field of ['dayOfMonth', 'dayOfWeek', 'month'] as const) {
    const pieces = byField.get(field);
    if (!isEvery(pieces)) clauses.push(clause(speech, field, pieces ?? []));
  }
  const sentence = clauses.join(', ');
  return sentence.charAt(0).toUpperCase() + sentence.slice(1);
}

function piecePhrase(speech: Speech, field: CronField, piece: Piece): string[] {
  const t = speech.t;
  switch (piece.kind) {
    case 'range':
      return [
        t(`${SAY}.${field}.range`, {
          from: format(speech, field, piece.from),
          to: format(speech, field, piece.to),
        }),
      ];
    case 'step':
      return [t(`${SAY}.${field}.step`, { step: piece.step })];
    case 'steppedRange':
      return [
        t(`${SAY}.${field}.steppedRange`, {
          step: piece.step,
          from: format(speech, field, piece.from),
          to: format(speech, field, piece.to),
        }),
      ];
    case 'lastDayOfMonth':
      return [t(`${SAY}.dayOfMonth.last`)];
    case 'nearestWeekday':
      return [t(`${SAY}.dayOfMonth.nearest`, { day: format(speech, 'dayOfMonth', piece.day) })];
    case 'lastWeekday':
      return [t(`${SAY}.dayOfWeek.last`, { day: format(speech, 'dayOfWeek', piece.weekday) })];
    case 'nth':
      return [
        t(`${SAY}.dayOfWeek.nth`, {
          nth: t(`tools.cron.nths.${piece.nth}`),
          day: format(speech, 'dayOfWeek', piece.weekday),
        }),
      ];
    case 'every':
    case 'value':
      return [];
  }
}
