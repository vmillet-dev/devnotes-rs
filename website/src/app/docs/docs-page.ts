import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  ElementRef,
  OnInit,
  afterNextRender,
  computed,
  inject,
  input,
  signal,
  viewChild,
} from '@angular/core';
import { RouterLink } from '@angular/router';

import { ScrollFrame } from '../core/scroll-frame';
import { Seo } from '../core/seo';
import { absolute } from '../core/site';
import { CHECKED_ON, DOC_PAGES } from './content/pages';
import { DOC_INDEX, DocEntry, docPath } from './doc-index';
import { DocBlocks } from './doc-blocks';

/** What an unknown slug falls back to: the overview. */
const OVERVIEW: DocEntry = { slug: '', nav: 'Documentation', tone: 'grey' };

/** A heading counts as read once it has passed this far below the header. */
const READ_LINE = 140;

@Component({
  selector: 'app-docs-page',
  imports: [RouterLink, DocBlocks],
  templateUrl: './docs-page.html',
  styleUrl: './docs-page.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class DocsPage implements OnInit {
  /** From the route's data: the page's path under `/docs`. */
  readonly slug = input('');

  private readonly seo = inject(Seo);
  protected readonly entries = DOC_INDEX;
  protected readonly checkedOn = CHECKED_ON;
  protected readonly path = docPath;
  protected readonly summaries = Object.fromEntries(DOC_PAGES.map((page) => [page.slug, page.summary]));

  protected readonly position = computed(() =>
    Math.max(
      0,
      DOC_INDEX.findIndex(({ slug }) => slug === this.slug()),
    ),
  );
  protected readonly entry = computed(() => DOC_INDEX.find(({ slug }) => slug === this.slug()) ?? OVERVIEW);
  protected readonly page = computed(
    () => DOC_PAGES.find(({ slug }) => slug === this.slug()) ?? DOC_PAGES[0],
  );
  protected readonly previous = computed(() => DOC_INDEX[this.position() - 1] ?? null);
  protected readonly next = computed(() => DOC_INDEX[this.position() + 1] ?? null);
  protected readonly sections = computed(() =>
    (this.page()?.body ?? []).flatMap((block) =>
      block.kind === 'h2' ? [{ id: block.id, text: block.text }] : [],
    ),
  );

  /** The sidebar, folded on a narrow screen. */
  protected readonly navOpen = signal(false);
  /** The section being read, marked in the contents beside the page. */
  protected readonly reading = signal<string | null>(null);
  private readonly article = viewChild.required<ElementRef<HTMLElement>>('article');

  constructor() {
    const frame = inject(ScrollFrame);
    const destroyRef = inject(DestroyRef);
    afterNextRender(() => {
      const headings = [...this.article().nativeElement.querySelectorAll<HTMLElement>('h2[id]')];
      const stop = frame.watch(() => {
        const passed = headings.filter((heading) => heading.getBoundingClientRect().top <= READ_LINE);
        this.reading.set(passed.at(-1)?.id ?? null);
      });
      destroyRef.onDestroy(stop);
    });
  }

  ngOnInit(): void {
    const page = this.page();
    if (!page) return;
    const path = docPath(page.slug);
    this.seo.describe({
      title: page.slug ? `${page.title} – Documentation de DevNotes` : 'Documentation de DevNotes',
      description: page.summary,
      path,
      data: {
        '@type': 'BreadcrumbList',
        itemListElement: [
          { '@type': 'ListItem', position: 1, name: 'Documentation', item: absolute('/docs') },
          ...(page.slug
            ? [{ '@type': 'ListItem', position: 2, name: page.title, item: absolute(path) }]
            : []),
        ],
      },
    });
  }

  protected toggleNav(): void {
    this.navOpen.update((open) => !open);
  }

  protected rgb(tone: string): string {
    return `var(--${tone}-rgb)`;
  }
}
