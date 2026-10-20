import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { VariableRow } from '@core/model/http-environment.model';
import { FakeHttpEnvironmentsRepository, variable } from '@testing/fake-http-environments-repository';
import { provideAppTesting } from '@testing/testing.providers';
import { VariablesTableComponent } from './variables-table.component';

const ROWS: readonly VariableRow[] = [
  variable('baseUrl', 'https://api.exemple.fr'),
  variable('clientSecret', 's3cr3t', { kind: 'secret' }),
];

describe('VariablesTableComponent', () => {
  let fixture: ComponentFixture<VariablesTableComponent>;
  let emitted: (readonly VariableRow[])[];
  let environments: FakeHttpEnvironmentsRepository;

  async function render(rows: readonly VariableRow[]) {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [VariablesTableComponent],
      providers: [provideAppTesting({ httpEnvironmentsRepository: environments })],
    });
    fixture = TestBed.createComponent(VariablesTableComponent);
    fixture.componentRef.setInput('rows', rows);
    fixture.componentRef.setInput('label', 'Variables');
    fixture.componentRef.setInput('kind', 'environment');
    emitted = [];
    fixture.componentInstance.rowsChange.subscribe((next) => {
      emitted.push(next);
      fixture.componentRef.setInput('rows', next);
    });
    fixture.autoDetectChanges();
    await fixture.whenStable();
  }

  beforeEach(async () => {
    environments = new FakeHttpEnvironmentsRepository();
    await render(ROWS);
  });

  const all = <E extends HTMLElement = HTMLElement>(selector: string): E[] => [
    ...fixture.nativeElement.querySelectorAll(selector),
  ];
  const type = (field: HTMLInputElement, text: string) => {
    field.value = text;
    field.dispatchEvent(new Event('input'));
  };

  it('draws each variable and a blank row to add one', () => {
    expect(all('[data-testid="http-variable-row"]')).toHaveLength(3);
    expect(all<HTMLInputElement>('[data-testid="http-variable-name"]').map((field) => field.value)).toEqual([
      'baseUrl',
      'clientSecret',
      '',
    ]);
  });

  it('masks a secret, and shows that one alone when asked', async () => {
    const values = () =>
      all<HTMLInputElement>('[data-testid="http-variable-value"]').map((field) => field.type);
    expect(values()).toEqual(['text', 'password', 'text']);

    all('[data-testid="http-variable-reveal"]')[0]?.click();
    await fixture.whenStable();

    expect(values()).toEqual(['text', 'text', 'text']);
    expect(all('[data-testid="http-variable-reveal"]')[0]?.getAttribute('aria-pressed')).toBe('true');
  });

  it('turns what is typed in the blank row into a variable', async () => {
    type(all<HTMLInputElement>('[data-testid="http-variable-name"]')[2]!, 'page');
    await fixture.whenStable();

    expect(emitted.at(-1)).toEqual([...ROWS, variable('page', '')]);
  });

  it('changes a type from its menu', async () => {
    all('[data-testid="http-variable-kind"]')[0]?.click();
    await fixture.whenStable();
    const secret = [...document.querySelectorAll<HTMLElement>('[data-testid="choice-option"]')].find(
      (option) => option.dataset['optionId'] === 'secret',
    );
    secret?.click();
    await fixture.whenStable();

    expect(emitted.at(-1)?.[0]?.kind).toBe('secret');
    expect(all<HTMLInputElement>('[data-testid="http-variable-value"]')[0]?.type).toBe('password');
  });

  it('switches a row off and removes another', async () => {
    all<HTMLInputElement>('[data-testid="http-variable-enabled"]')[0]?.click();
    await fixture.whenStable();
    expect(emitted.at(-1)?.[0]?.enabled).toBe(false);

    all('[data-testid="http-variable-remove"]')[1]?.click();
    await fixture.whenStable();
    expect(emitted.at(-1)?.map((row) => row.name)).toEqual(['baseUrl']);
  });

  it('marks a row by the fault Rust names, and says why under the table', async () => {
    environments.faults = [null, 'duplicate'];
    await render([variable('a', '1'), variable('a', '2')]);

    await vi.waitFor(() => expect(all('[data-testid="http-variable-fault"]')).toHaveLength(1));
    expect(all('[data-testid="http-variable-row"]')[1]?.dataset['fault']).toBe('duplicate');
    expect(all('[data-testid="http-variable-fault"]')[0]?.dataset['fault']).toBe('duplicate');
  });

  it('says which request fills a variable, and opens it when it is saved', async () => {
    await render([
      variable('accessToken', 'eyJ', { kind: 'secret', filledBy: { requestId: 'login', name: 'Login' } }),
      variable('draftValue', '1', { filledBy: { requestId: null, name: 'Nouvelle requête' } }),
    ]);
    const opened: unknown[] = [];
    fixture.componentInstance.fillerOpened.subscribe((by) => opened.push(by));
    const tags = [
      ...(fixture.nativeElement as HTMLElement).querySelectorAll<HTMLButtonElement>(
        '[data-testid="http-variable-filled"]',
      ),
    ];
    expect(tags.map((tag) => tag.textContent?.trim())).toEqual([
      'remplie par Login',
      'remplie par Nouvelle requête',
    ]);
    expect(
      [...(fixture.nativeElement as HTMLElement).querySelectorAll('[data-testid="http-variable-from"]')].map(
        (from) => from.textContent?.trim(),
      ),
    ).toEqual(['depuis Login', 'depuis Nouvelle requête']);
    expect(tags[1]!.disabled).toBe(true);
    tags[0]!.click();
    expect(opened).toEqual([{ requestId: 'login', name: 'Login' }]);
  });
});
