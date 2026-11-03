import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

/**
 * Golden corpora (migration plan § 8.6): what the JavaScript libraries produce today, written
 * under `golden/<subject>/` for the Rust that replaces them to reproduce, or to list its gaps
 * against. Data files, not assertions: a `*.golden.spec.ts` does nothing unless
 * `npm run golden` set `GOLDEN_WRITE`.
 */
export interface GoldenCase {
  readonly input: unknown;
  readonly options?: unknown;
  readonly output: unknown;
}

declare const process: { readonly env: Record<string, string | undefined>; cwd(): string };

export const writingGolden = process.env['GOLDEN_WRITE'] === '1';

/** Pretty JSON, one trailing newline: a second run writes the same bytes. */
export function writeGolden(subject: string, cases: readonly GoldenCase[], file = 'cases.json'): void {
  const directory = join(process.cwd(), 'golden', subject);
  mkdirSync(directory, { recursive: true });
  writeFileSync(join(directory, file), `${JSON.stringify(cases, null, 2)}\n`);
}

/** A binary or text output beside the cases, named by its case. */
export function writeGoldenFile(subject: string, name: string, content: string | Uint8Array): void {
  const directory = join(process.cwd(), 'golden', subject);
  mkdirSync(directory, { recursive: true });
  writeFileSync(join(directory, name), content);
}
