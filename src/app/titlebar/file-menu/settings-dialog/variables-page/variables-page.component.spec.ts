import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { FakeNotesRepository } from '@testing/fake-notes-repository';
import { VariablesStore } from '@core/state/variables.store';
import { provideAppTesting } from '@testing/testing.providers';
import { VariablesPageComponent } from './variables-page.component';

describe('VariablesPageComponent', () => {
  let fixture: ComponentFixture<VariablesPageComponent>;
  let repository: FakeNotesRepository;
  let store: VariablesStore;

  function rows(): HTMLElement[] {
    return [...fixture.nativeElement.querySelectorAll('.variable-row')];
  }

  function nameInput(index: number): HTMLInputElement {
    return rows()[index].querySelector('.variable-name')!;
  }

  function valueInput(index: number): HTMLInputElement {
    return rows()[index].querySelector('.variable-value')!;
  }

  function type(input: HTMLInputElement, text: string): void {
    input.value = text;
    input.dispatchEvent(new Event('input'));
  }

  async function render(): Promise<void> {
    fixture = TestBed.createComponent(VariablesPageComponent);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  }

  beforeEach(() => {
    TestBed.resetTestingModule();
    repository = new FakeNotesRepository();
    TestBed.configureTestingModule({
      imports: [VariablesPageComponent],
      providers: [provideAppTesting({ notesRepository: repository })],
    });
    store = TestBed.inject(VariablesStore);
  });

  it('lists what is already stored', async () => {
    await repository.saveVariables({ host: 'db.internal' });

    await render();

    expect(nameInput(0).value).toBe('host');
    expect(valueInput(0).value).toBe('db.internal');
  });

  it('says so rather than showing an empty grid', async () => {
    await render();

    expect(fixture.nativeElement.querySelector('.variables-empty')).not.toBeNull();
    expect(rows()).toHaveLength(0);
  });

  /** Nothing here reaches the corpus before the panel's own button says so. */
  it('holds what was typed until the panel commits it', async () => {
    await render();
    fixture.nativeElement.querySelector('.variables-add').click();
    await fixture.whenStable();

    type(nameInput(0), 'host');
    type(valueInput(0), 'db.internal');
    valueInput(0).dispatchEvent(new Event('blur'));
    await fixture.whenStable();

    expect(store.isDirty()).toBe(true);
    expect(await repository.loadVariables()).toEqual({});

    await store.commit();

    expect(await repository.loadVariables()).toEqual({ host: 'db.internal' });
  });

  it('marks a name no token could ever carry', async () => {
    await render();
    fixture.nativeElement.querySelector('.variables-add').click();
    await fixture.whenStable();

    type(nameInput(0), 'user.name');
    await fixture.whenStable();

    expect(nameInput(0).getAttribute('aria-invalid')).toBe('true');
    expect(nameInput(0).classList.contains('rejected')).toBe(true);
  });

  it('leaves a freshly added row unmarked', async () => {
    await render();

    fixture.nativeElement.querySelector('.variables-add').click();
    await fixture.whenStable();

    expect(nameInput(0).getAttribute('aria-invalid')).toBeNull();
  });

  it('warns when two rows would fight over the same name', async () => {
    await repository.saveVariables({ host: 'db.internal' });
    await render();
    fixture.nativeElement.querySelector('.variables-add').click();
    await fixture.whenStable();

    type(nameInput(1), 'host');
    await fixture.whenStable();

    expect(fixture.nativeElement.querySelector('.variables-warning').textContent).toContain('host');
  });

  it('removes a row from the list, and from the corpus once it is committed', async () => {
    await repository.saveVariables({ host: 'db.internal' });
    await render();

    rows()[0].querySelector<HTMLButtonElement>('.variable-remove')!.click();
    await fixture.whenStable();

    expect(rows()).toHaveLength(0);
    expect(await repository.loadVariables()).toEqual({ host: 'db.internal' });

    await store.commit();

    expect(await repository.loadVariables()).toEqual({});
  });
});
