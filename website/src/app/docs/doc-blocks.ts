import { ChangeDetectionStrategy, Component, computed, input, signal } from '@angular/core';
import { RouterLink } from '@angular/router';

import { DOC_INDEX, docPath } from './doc-index';
import { Block, ShortcutRow, Text } from './doc.model';
import { DocText } from './doc-text';

/** Every block in one shape, so the template reads its fields without narrowing. */
interface View {
  readonly kind: Block['kind'];
  readonly id: string;
  readonly heading: string;
  readonly text: Text;
  readonly ordered: boolean;
  readonly items: readonly Text[];
  readonly title: string;
  readonly lang: string;
  readonly code: string;
  readonly tone: 'tip' | 'warn';
  readonly caption: string;
  readonly shortcuts: readonly ShortcutRow[];
  readonly head: readonly string[];
  readonly rows: readonly (readonly Text[])[];
}

const EMPTY: Omit<View, 'kind'> = {
  id: '',
  heading: '',
  text: [],
  ordered: false,
  items: [],
  title: '',
  lang: '',
  code: '',
  tone: 'tip',
  caption: '',
  shortcuts: [],
  head: [],
  rows: [],
};

function view(block: Block): View {
  switch (block.kind) {
    case 'p':
      return { ...EMPTY, kind: block.kind, text: block.text };
    case 'h2':
    case 'h3':
      return { ...EMPTY, kind: block.kind, id: block.id, heading: block.text };
    case 'list':
      return { ...EMPTY, kind: block.kind, ordered: block.ordered, items: block.items };
    case 'code':
      return { ...EMPTY, kind: block.kind, title: block.title, lang: block.lang, code: block.code };
    case 'note':
      return { ...EMPTY, kind: block.kind, tone: block.tone, text: block.text };
    case 'keys':
      return { ...EMPTY, kind: block.kind, caption: block.caption, shortcuts: block.rows };
    case 'table':
      return { ...EMPTY, kind: block.kind, caption: block.caption, head: block.head, rows: block.rows };
    case 'guides':
      return { ...EMPTY, kind: block.kind };
  }
}

/** The colour a code block's language chip takes, as on a card. */
const CHIPS: Record<string, string> = { sh: 'l-sh', sql: 'l-sql', json: 'l-json', ts: 'l-ts', yml: 'l-yml' };

/** A page's body, block by block. No `innerHTML` anywhere: every word is bound as text. */
@Component({
  selector: 'app-doc-blocks',
  imports: [RouterLink, DocText],
  templateUrl: './doc-blocks.html',
  styleUrl: './doc-blocks.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class DocBlocks {
  readonly blocks = input.required<readonly Block[]>();
  /** What each other page is about, for the overview's list. */
  readonly summaries = input<Readonly<Record<string, string>>>({});

  protected readonly views = computed(() => this.blocks().map(view));
  protected readonly guides = DOC_INDEX.filter(({ slug }) => slug);
  protected readonly path = docPath;
  /** The code block whose copy was just taken, by position. */
  protected readonly copied = signal<number | null>(null);

  protected chip(lang: string): string {
    return CHIPS[lang] ?? '';
  }

  protected rgb(tone: string): string {
    return `var(--${tone}-rgb)`;
  }

  protected copy(index: number, code: string, target: HTMLElement): void {
    const done = () => {
      this.copied.set(index);
      setTimeout(() => this.copied.update((shown) => (shown === index ? null : shown)), 2000);
    };
    // Where the clipboard is refused, the code is selected for the visitor to copy.
    const select = () => {
      const range = document.createRange();
      range.selectNodeContents(target);
      const selection = getSelection();
      selection?.removeAllRanges();
      selection?.addRange(range);
    };
    if (!navigator.clipboard) return select();
    navigator.clipboard.writeText(code).then(done, select);
  }
}
