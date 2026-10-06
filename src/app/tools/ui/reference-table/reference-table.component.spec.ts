import { ChangeDetectionStrategy, Component, signal } from '@angular/core';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { provideAppTesting } from '@testing/testing.providers';
import { FakeClipboard } from '@testing/fake-clipboard';
import {
  ReferenceCellsDirective,
  ReferenceColumn,
  ReferenceGroup,
  ReferenceRow,
  ReferenceTableComponent,
} from './reference-table.component';

interface Fruit extends ReferenceRow {
  readonly name: string;
}

const fruit = (key: string, group: string, name: string): Fruit => ({
  key,
  group,
  name,
  searchable: [key, name],
});

const FRUITS: readonly Fruit[] = [
  fruit('1', 'red', 'Fraise'),
  fruit('2', 'red', 'Cerise'),
  fruit('3', 'yellow', 'Citron'),
  fruit('4', 'yellow', 'Ananas'),
  fruit('5', 'green', 'Kiwi épicé'),
];

const GROUPS: readonly ReferenceGroup[] = [
  { id: 'red', label: 'Rouges' },
  { id: 'yellow', label: 'Jaunes' },
  { id: 'green', label: 'Verts' },
];

const COLUMNS: readonly ReferenceColumn[] = [
  { id: 'key', labelKey: 'tools.http-status.columns.code' },
  { id: 'name', labelKey: 'tools.http-status.columns.name' },
];

/** Reads a query its own way: case kept, accents kept. */
@Component({
  imports: [ReferenceCellsDirective, ReferenceTableComponent],
  template: `
    <app-reference-table
      [rows]="rows"
      [groups]="groups"
      [columns]="columns"
      [matches]="matches"
      caption="Fruits"
      placeholder="Un fruit"
      [(query)]="query"
    >
      <ng-template [appReferenceCells]="rows" let-row>
        <td data-testid="fruit-key">{{ row.key }}</td>
        <td data-testid="fruit-name">{{ row.name }}</td>
      </ng-template>
    </app-reference-table>
  `,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
class HostComponent {
  readonly rows = FRUITS;
  readonly groups = GROUPS;
  readonly columns = COLUMNS;
  readonly query = signal('');
  readonly matches = (row: Fruit, query: string) => row.searchable.some((text) => text.includes(query));
}

@Component({
  imports: [ReferenceCellsDirective, ReferenceTableComponent],
  template: `
    <app-reference-table
      [rows]="rows"
      [groups]="groups"
      [columns]="columns"
      caption="Fruits"
      placeholder="Un fruit"
    >
      <ng-template [appReferenceCells]="rows" let-row>
        <td data-testid="fruit-key">{{ row.key }}</td>
        <td data-testid="fruit-name">{{ row.name }}</td>
      </ng-template>
    </app-reference-table>
  `,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
class FoldingHostComponent {
  readonly rows = FRUITS;
  readonly groups = GROUPS;
  readonly columns = COLUMNS;
}

describe('ReferenceTableComponent', () => {
  let fixture: ComponentFixture<HostComponent | FoldingHostComponent>;
  let clipboard: FakeClipboard;

  async function render(host: typeof HostComponent | typeof FoldingHostComponent): Promise<void> {
    TestBed.resetTestingModule();
    clipboard = new FakeClipboard();
    TestBed.configureTestingModule({ imports: [host], providers: [provideAppTesting({ clipboard })] });
    fixture = TestBed.createComponent(host);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  }

  beforeEach(() => render(HostComponent));

  const host = () => fixture.componentInstance as HostComponent;

  const all = (selector: string): HTMLElement[] => [...fixture.nativeElement.querySelectorAll(selector)];
  const one = <E extends HTMLElement = HTMLElement>(selector: string): E =>
    fixture.nativeElement.querySelector(selector);
  const keys = () => all('[data-testid="reference-row"]').map((row) => row.dataset['key']);
  const groups = () => all('[data-testid="reference-group"]').map((group) => group.dataset['group']);
  const count = () => one('[data-testid="reference-count"]').textContent?.trim();

  async function search(query: string): Promise<void> {
    const field = one<HTMLInputElement>('[data-testid="reference-search"]');
    field.value = query;
    field.dispatchEvent(new Event('input'));
    await fixture.whenStable();
  }

  it('draws every group as a table of its own, named, its columns named', () => {
    expect(keys()).toEqual(['1', '2', '3', '4', '5']);
    expect(groups()).toEqual(['red', 'yellow', 'green']);
    expect(all('[data-group="red"] thead th[scope="col"]').map((th) => th.textContent?.trim())).toEqual([
      'Code',
      'Nom',
      'Copier',
    ]);
    expect(all('[data-testid="reference-group"] table')).toHaveLength(3);
    expect(all('caption').map((caption) => caption.textContent?.trim())).toEqual([
      'Rouges2',
      'Jaunes2',
      'Verts1',
    ]);
    expect(one('[data-testid="reference-table"]').getAttribute('aria-label')).toBe('Fruits');
    expect(all('[data-testid="fruit-name"]').map((cell) => cell.textContent)).toContain('Citron');
    expect(count()).toBe('5 entrées');
  });

  it('filters as it is typed, with the reference’s own reading, and counts what is left', async () => {
    await search('C');

    expect(keys()).toEqual(['2', '3']);
    expect(groups()).toEqual(['red', 'yellow']);
    expect(count()).toBe('2 entrées sur 5');
    expect(host().query()).toBe('C');
  });

  it('folds case and accents when the reference has no reading of its own', async () => {
    await render(FoldingHostComponent);

    await search('EPICE');

    expect(keys()).toEqual(['5']);
  });

  it('says so when nothing matches, and draws no empty table', async () => {
    await search('zzz');

    expect(one('[data-testid="reference-table"]')).toBeNull();
    expect(one('[data-testid="reference-empty"]').textContent).toContain('« zzz »');
    expect(count()).toBe('Aucune entrée sur 5');
  });

  it('keeps the focus in the field while the rows change under it', async () => {
    const field = one<HTMLInputElement>('[data-testid="reference-search"]');
    field.focus();

    await search('Fr');
    await search('Fra');

    expect(document.activeElement).toBe(field);
    expect(one('[data-testid="reference-search"]')).toBe(field);
  });

  it('starts from the query it is handed, found again with the tool', async () => {
    host().query.set('Kiwi');
    await fixture.whenStable();

    expect(keys()).toEqual(['5']);
    expect(one<HTMLInputElement>('[data-testid="reference-search"]').value).toBe('Kiwi');
  });

  it("copies a row's key", async () => {
    one<HTMLButtonElement>('[data-key="3"] [data-testid="copy-value"]').click();
    await fixture.whenStable();

    await expect.poll(() => clipboard.content).toBe('3');
  });
});
