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
import { Router, RouterLink } from '@angular/router';

import { ScrollFrame } from '../core/scroll-frame';
import { Seo } from '../core/seo';
import { absolute } from '../core/site';
import { CHECKED_ON, DOC_PAGES } from './content/pages';
import { DOC_GROUPS, DOC_INDEX, DocEntry, docPath } from './doc-index';
import { search, searchIndex } from './doc-search';
import { DocBlocks } from './doc-blocks';

/** What an unknown slug falls back to: the overview. */
const OVERVIEW: DocEntry = { slug: '', nav: 'Documentation', tone: 'grey', group: 'Démarrer' };

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
  private readonly router = inject(Router);
  protected readonly groups = DOC_GROUPS;
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

  /** What the visitor looks for, across every page; empty shows the sidebar. */
  protected readonly query = signal('');
  private readonly index = searchIndex(DOC_PAGES);
  protected readonly results = computed(() => search(this.index, this.query()));
  protected readonly status = computed(() => {
    const query = this.query().trim();
    if (!query) return '';
    const found = this.results().length;
    if (!found) return `Aucune section ne parle de « ${query} ».`;
    return found === 1 ? 'Une section' : `${found} sections`;
  });

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

  protected onQuery(event: Event): void {
    this.query.set((event.target as HTMLInputElement).value);
  }

  /** Enter opens the first result, Escape empties the field. */
  protected onKey(event: KeyboardEvent): void {
    if (event.key === 'Escape') {
      this.clear();
      return;
    }
    const first = this.results()[0];
    if (event.key !== 'Enter' || !first) return;
    event.preventDefault();
    const { slug, anchor } = first.entry;
    void this.router.navigate([docPath(slug)], anchor ? { fragment: anchor } : {});
    this.clear();
  }

  protected clear(): void {
    this.query.set('');
  }

  protected toggleNav(): void {
    this.navOpen.update((open) => !open);
  }

  protected rgb(tone: string): string {
    return `var(--${tone}-rgb)`;
  }
}
