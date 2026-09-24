import { Injectable, Injector, inject } from '@angular/core';
import { DefaultTranspiler, TranslocoService } from '@jsverse/transloco';
import type { Translation } from '@jsverse/transloco';

/**
 * Counting, in the one ICU shape these translations use:
 * `{name, plural, =0 {…} one {# thing} other {# things}}`.
 *
 * Not `@jsverse/transloco-messageformat`, which compiles each message with `new Function`:
 * the CSP is `script-src 'self'`, and jsdom, having no policy, cannot show it failing. Tiny on
 * purpose — `plural`, `=N`, the `Intl.PluralRules` categories and `#`; no `select`, no
 * nesting. `{{name}}` stays `DefaultTranspiler`'s.
 */
@Injectable()
export class PluralTranspiler extends DefaultTranspiler {
  private readonly injector = inject(Injector);
  private service?: TranslocoService;

  override transpile(payload: {
    value: unknown;
    params?: Translation;
    translation: Translation;
    key: string;
  }): unknown {
    const { value, params } = payload;

    return typeof value === 'string'
      ? super.transpile({ ...payload, value: expandPlurals(value, params ?? {}, this.locale()) })
      : super.transpile(payload);
  }

  /**
   * ⚠️ Resolved on the first render rather than injected: `TranslocoService` asks for the
   * transpiler in its own constructor, so taking it here would be a cycle.
   */
  private locale(): string {
    this.service ??= this.injector.get(TranslocoService);
    return this.service.getActiveLang();
  }
}

/** Where a `{name, plural,` block starts. The name is what the parameters are keyed by. */
const PLURAL_HEAD = /\{\s*([A-Za-z0-9_]+)\s*,\s*plural\s*,/g;

export function expandPlurals(source: string, params: Translation, locale: string): string {
  PLURAL_HEAD.lastIndex = 0;
  let out = '';
  let from = 0;
  let head: RegExpExecArray | null;

  while ((head = PLURAL_HEAD.exec(source)) !== null) {
    const closing = matchingBrace(source, head.index);
    // An unbalanced block is left exactly as written: rendering half a sentence would be
    // worse than rendering the source of it, which is at least visibly wrong.
    if (closing < 0) break;

    const count = Number(params[head[1] ?? '']);
    const branches = source.slice(head.index + head[0].length, closing);

    out += source.slice(from, head.index) + choose(branches, count, locale);
    from = closing + 1;
    PLURAL_HEAD.lastIndex = from;
  }

  return out + source.slice(from);
}

/** The index of the `}` closing the `{` at `start`, or `-1`. */
function matchingBrace(source: string, start: number): number {
  let depth = 0;

  for (let at = start; at < source.length; at += 1) {
    if (source[at] === '{') depth += 1;
    else if (source[at] === '}') {
      depth -= 1;
      if (depth === 0) return at;
    }
  }

  return -1;
}

/**
 * An exact `=N` wins over a category, which is the whole reason `=0` is written out:
 * French calls zero `one`, so "0 note" would otherwise read as a singular where the
 * sentence wants "aucune note".
 */
function choose(branches: string, count: number, locale: string): string {
  const cases = parseBranches(branches);
  const category = Number.isFinite(count) ? new Intl.PluralRules(locale).select(count) : 'other';
  const chosen = cases.get(`=${count}`) ?? cases.get(category) ?? cases.get('other') ?? '';

  return chosen.replaceAll('#', Number.isFinite(count) ? String(count) : '');
}

/** `selector {text}` pairs, in order, with the braces inside each text left alone. */
function parseBranches(branches: string): Map<string, string> {
  const cases = new Map<string, string>();
  let at = 0;

  while (at < branches.length) {
    const open = branches.indexOf('{', at);
    if (open < 0) break;

    const selector = branches.slice(at, open).trim();
    const closing = matchingBrace(branches, open);
    if (closing < 0) break;

    if (selector.length > 0) {
      cases.set(selector, branches.slice(open + 1, closing));
    }
    at = closing + 1;
  }

  return cases;
}
