import { InjectionToken, Injectable, inject } from '@angular/core';
import type { Mermaid, MermaidConfig } from 'mermaid';

/** The fence language that makes a Note's code block a diagram. */
export const MERMAID = 'mermaid';

/** A diagram drawn, or why it could not be: the line Mermaid stopped at, when it says. */
export type DiagramResult =
  { readonly svg: string } | { readonly error: string; readonly line: number | null };

export type MermaidApi = Pick<Mermaid, 'initialize' | 'render'>;

/** A chunk of its own, loaded the first time a Note holds a diagram. */
export const MERMAID_LOADER = new InjectionToken<() => Promise<MermaidApi>>('MERMAID_LOADER', {
  providedIn: 'root',
  factory: () => () => import('mermaid').then((module) => module.default),
});

/** Drawings kept, so a Note reopened or a keystroke undone draws at once. */
const REMEMBERED = 64;

/**
 * Draws a Mermaid diagram in the app's colours, at its `strict` level: what it returns is
 * sanitised by its own DOMPurify.
 */
@Injectable({ providedIn: 'root' })
export class MermaidRenderer {
  private readonly load = inject(MERMAID_LOADER);
  private api: Promise<MermaidApi> | null = null;
  private queue: Promise<unknown> = Promise.resolve();
  private readonly drawn = new Map<string, DiagramResult>();
  private configured: string | null = null;
  private count = 0;

  /**
   * ⚠️ One drawing at a time: Mermaid's configuration is global, and so is the element it
   * measures text in.
   */
  render(source: string): Promise<DiagramResult> {
    const theme = document.documentElement.dataset['theme'] ?? 'dark';
    const key = `${theme}\u0000${source}`;
    const known = this.drawn.get(key);
    if (known) return Promise.resolve(known);

    const next = this.queue.then(() => this.draw(source, theme, key));
    this.queue = next.catch(() => undefined);
    return next;
  }

  private async draw(source: string, theme: string, key: string): Promise<DiagramResult> {
    const api = await (this.api ??= this.load());
    if (this.configured !== theme) {
      api.initialize(configFor(getComputedStyle(document.documentElement), theme !== 'light'));
      this.configured = theme;
    }

    let result: DiagramResult;
    try {
      const { svg } = await api.render(`devnotes-diagram-${++this.count}`, source);
      result = { svg };
    } catch (error) {
      result = { error: detailOf(error), line: lineOf(error) };
    }
    this.drawn.set(key, result);
    if (this.drawn.size > REMEMBERED) {
      this.drawn.delete(this.drawn.keys().next().value ?? key);
    }
    return result;
  }
}

/** The tokens of the theme on screen: Mermaid derives its shades from concrete colours. */
function configFor(style: CSSStyleDeclaration, dark: boolean): MermaidConfig {
  const token = (name: string): string => style.getPropertyValue(name).trim();
  const font = token('--font-sans');
  return {
    startOnLoad: false,
    securityLevel: 'strict',
    suppressErrorRendering: true,
    theme: 'base',
    fontFamily: font,
    themeVariables: {
      darkMode: dark,
      fontFamily: font,
      background: token('--bg-1'),
      primaryColor: token('--bg-3'),
      primaryTextColor: token('--text-0'),
      primaryBorderColor: token('--line'),
      secondaryColor: token('--bg-2'),
      tertiaryColor: token('--bg-2'),
      lineColor: token('--text-2'),
      textColor: token('--text-0'),
      edgeLabelBackground: token('--bg-1'),
    },
  };
}

function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/**
 * What Mermaid found, without the line its message opens on: counted past the newline it adds
 * to the source, it can name one the block does not have, and the line is said beside it.
 */
function detailOf(error: unknown): string {
  return messageOf(error).replace(/^Parse error on line \d+:\n/, '');
}

/** Its parsers say it two ways: a location in the error, or a line in its message. */
export function lineOf(error: unknown): number | null {
  const hash = (error as { hash?: { loc?: { first_line?: unknown } } } | null)?.hash;
  const first = hash?.loc?.first_line;
  if (typeof first === 'number') return first;

  const match = /\bline (\d+)/i.exec(messageOf(error));
  return match ? Number(match[1]) : null;
}
