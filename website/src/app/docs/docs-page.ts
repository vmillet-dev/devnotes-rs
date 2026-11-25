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

import { Lang } from '../core/lang';
import { Language } from '../core/language';
import { ScrollFrame } from '../core/scroll-frame';
import { Seo } from '../core/seo';
import { absolute } from '../core/site';
import { CHECKED_ON, DOC_PAGES } from './content/pages';
import { DOC_INDEX, docGroups, docPath } from './doc-index';
import { search, searchIndex } from './doc-search';
import { DocBlocks } from './doc-blocks';

/** A heading counts as read once it has passed this far below the header. */
const READ_LINE = 140;

const FR = {
  docs: 'Documentation',
  title: (page: string) => `${page} – Documentation de DevNotes`,
  home: 'Documentation de DevNotes',
  search: 'Rechercher dans la documentation',
  placeholder: 'Rechercher dans la doc',
  none: (query: string) => `Aucune section ne parle de « ${query} ».`,
  found: (count: number) => (count === 1 ? 'Une section' : `${count} sections`),
  crumbs: 'Fil d’Ariane',
  checked: 'Vérifié sur DevNotes',
  pager: 'Pages voisines',
  previous: 'Précédent',
  next: 'Suivant',
  toc: 'Sur cette page',
};

const EN: typeof FR = {
  docs: 'Documentation',
  title: (page: string) => `${page} – DevNotes documentation`,
  home: 'DevNotes documentation',
  search: 'Search the documentation',
  placeholder: 'Search the docs',
  none: (query: string) => `No section mentions “${query}”.`,
  found: (count: number) => (count === 1 ? 'One section' : `${count} sections`),
  crumbs: 'Breadcrumb',
  checked: 'Checked on DevNotes',
  pager: 'Neighbouring pages',
  previous: 'Previous',
  next: 'Next',
  toc: 'On this page',
};

const TEXT: Readonly<Record<Lang, typeof FR>> = { fr: FR, en: EN };

@Component({
  selector: 'app-docs-page',
  imports: [RouterLink, DocBlocks],
  templateUrl: './docs-page.html',
  styleUrl: './docs-page.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class DocsPage implements OnInit {
  /** From the route's data: the page's id in `DOC_INDEX`. */
  readonly doc = input('overview');

  private readonly seo = inject(Seo);
  private readonly router = inject(Router);
  protected readonly lang = inject(Language).lang;
  protected readonly text = computed(() => TEXT[this.lang()]);
  protected readonly groups = computed(() => docGroups(this.lang()));
  protected readonly checkedOn = CHECKED_ON;
  private readonly pages = computed(() => DOC_PAGES[this.lang()]);
  protected readonly summaries = computed(() =>
    Object.fromEntries(this.pages().map((page) => [page.id, page.summary])),
  );

  protected readonly position = computed(() =>
    Math.max(
      0,
      DOC_INDEX.findIndex(({ id }) => id === this.doc()),
    ),
  );
  protected readonly entry = computed(() => DOC_INDEX[this.position()] ?? DOC_INDEX[0]);
  protected readonly page = computed(() => this.pages().find(({ id }) => id === this.entry()?.id));
  protected readonly previous = computed(() => DOC_INDEX[this.position() - 1] ?? null);
  protected readonly next = computed(() => DOC_INDEX[this.position() + 1] ?? null);
  protected readonly sections = computed(() =>
    (this.page()?.body ?? []).flatMap((block) =>
      block.kind === 'h2' ? [{ id: block.id, text: block.text }] : [],
    ),
  );

  /** What the visitor looks for, across every page; empty shows the sidebar. */
  protected readonly query = signal('');
  private readonly index = computed(() => searchIndex(this.pages()));
  protected readonly results = computed(() => search(this.index(), this.query()));
  protected readonly status = computed(() => {
    const query = this.query().trim();
    if (!query) return '';
    const found = this.results().length;
    return found ? this.text().found(found) : this.text().none(query);
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
    const lang = this.lang();
    const text = TEXT[lang];
    const overview = page.id === 'overview';
    const path = docPath(page.id, lang);
    this.seo.describe({
      title: overview ? text.home : text.title(page.title),
      description: page.summary,
      path,
      lang,
      data: {
        '@type': 'BreadcrumbList',
        itemListElement: [
          { '@type': 'ListItem', position: 1, name: text.docs, item: absolute(docPath('overview', lang)) },
          ...(overview ? [] : [{ '@type': 'ListItem', position: 2, name: page.title, item: absolute(path) }]),
        ],
      },
    });
  }

  protected path(id: string): string {
    return docPath(id, this.lang());
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
    const { id, anchor } = first.entry;
    void this.router.navigate([this.path(id)], anchor ? { fragment: anchor } : {});
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
