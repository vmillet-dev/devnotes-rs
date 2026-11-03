import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { describe, it } from 'vitest';
import type { MapNode } from '@core/model/note.model';
import { LANGUAGES, LanguageTag } from '@core/model/language.model';
import { GRAMMARS, loadGrammar } from '@core/services/syntax/grammars';
import { highlightRanges } from '@core/services/syntax/highlighter';
import { GoldenCase, writeGolden, writingGolden } from '@testing/golden';
import { layoutMindMap } from './mind-map.layout';

declare const process: { cwd(): string };

const leaf = (text: string, slide: number): MapNode => ({ text, slide, children: [] });

/** A Note's outline as Rust slices it: one heading, several, nested, long words, a single branch. */
const OUTLINES: readonly { readonly title: string; readonly map: readonly MapNode[] }[] = [
  { title: 'Seule', map: [] },
  { title: 'Une branche', map: [leaf('Introduction', 1)] },
  {
    title: 'Déploiement',
    map: [
      { text: 'Préparer', slide: 1, children: [leaf('Sauvegarde', 1), leaf('Prévenir l’astreinte', 1)] },
      { text: 'Basculer', slide: 2, children: [leaf('DNS', 2)] },
      leaf('Vérifier', 3),
    ],
  },
  {
    title: 'Profond',
    map: [
      {
        text: 'Niveau un',
        slide: 1,
        children: [
          {
            text: 'Niveau deux',
            slide: 1,
            children: [{ text: 'Niveau trois', slide: 1, children: [leaf('Niveau quatre', 1)] }],
          },
        ],
      },
    ],
  },
  {
    title: 'Large',
    map: Array.from({ length: 9 }, (_, n) => leaf(`Branche ${n} au titre plutôt long pour un nœud`, n + 1)),
  },
];

/** One sample a language: the detection corpus of the core, by its language's name. */
function samples(): Map<string, string> {
  const directory = join(process.cwd(), 'crates', 'engine', 'tests', 'corpus');
  const found = new Map<string, string>();
  for (const file of readdirSync(directory)) {
    const tag = file.replace(/\.txt$/, '');
    if (!found.has(tag)) found.set(tag, readFileSync(join(directory, file), 'utf8'));
  }
  return found;
}

describe.runIf(writingGolden)('golden: presentation and highlighting', () => {
  it('captures where the mind map places each outline', () => {
    const cases: GoldenCase[] = OUTLINES.map(({ title, map }) => ({
      input: { title, map },
      output: layoutMindMap(title, map),
    }));
    writeGolden('mind-map', cases);
  });

  it('captures the tok-* classes each grammar puts on a sample of its language', async () => {
    const corpus = samples();
    const cases: GoldenCase[] = [];
    for (const tag of Object.keys(LANGUAGES) as LanguageTag[]) {
      const sample = corpus.get(tag);
      if (sample === undefined || GRAMMARS[tag] === null) continue;
      const grammar = await loadGrammar(tag);
      cases.push({
        input: { language: tag, text: sample },
        output: highlightRanges(sample, grammar?.language ?? null),
      });
    }
    writeGolden('syntax-tokens', cases);
  }, 120_000);
});
