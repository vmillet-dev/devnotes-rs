import { ComponentFixture, TestBed } from '@angular/core/testing';
import { By } from '@angular/platform-browser';
import { beforeEach, describe, expect, it } from 'vitest';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { TagManagerComponent } from './tag-manager.component';

const TAGS = [
  { tag: 'api', noteCount: 1 },
  { tag: 'auth', noteCount: 4 },
];

describe('TagManagerComponent', () => {
  let fixture: ComponentFixture<TagManagerComponent>;

  function items(): HTMLButtonElement[] {
    return [...fixture.nativeElement.querySelectorAll('.tags-item')];
  }

  function applyButton(): HTMLButtonElement {
    return fixture.nativeElement.querySelector('.tags-action');
  }

  function deleteButton(): HTMLButtonElement {
    return [...fixture.nativeElement.querySelectorAll('.tags-action')][1];
  }

  async function select(...tags: string[]): Promise<void> {
    fixture.componentRef.setInput('selected', new Set(tags));
    await fixture.whenStable();
  }

  async function propose(change: {
    kind: 'rename' | 'merge' | 'delete';
    tags: readonly string[];
    into: string;
    notes: number;
  }): Promise<void> {
    fixture.componentRef.setInput('pending', change);
    await fixture.whenStable();
  }

  async function type(value: string): Promise<void> {
    const input: HTMLInputElement = fixture.nativeElement.querySelector('.tags-target');
    input.value = value;
    input.dispatchEvent(new Event('input'));
    await fixture.whenStable();
  }

  beforeEach(async () => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [TagManagerComponent],
      providers: [provideTranslocoTesting()],
    });
    fixture = TestBed.createComponent(TagManagerComponent);
    fixture.componentRef.setInput('tags', TAGS);
    fixture.componentRef.setInput('selected', new Set<string>());
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  it('lists each tag with the number of notes carrying it', () => {
    expect(items().map((item) => item.textContent?.replace(/\s+/g, ' ').trim())).toEqual([
      '#api 1 note',
      '#auth 4 notes',
    ]);
  });

  it('exposes the selection as pressed toggles', async () => {
    await select('auth');

    expect(items().map((item) => item.getAttribute('aria-pressed'))).toEqual(['false', 'true']);
  });

  it('calls it a rename for one tag and a merge for several', async () => {
    await select('auth');
    expect(applyButton().textContent?.trim()).toBe('Renommer');

    await select('auth', 'api');
    expect(applyButton().textContent?.trim()).toBe('Fusionner');
  });

  it('marks its actions unavailable without a selection', () => {
    expect(applyButton().getAttribute('aria-disabled')).toBe('true');
    expect(deleteButton().getAttribute('aria-disabled')).toBe('true');
  });

  /** The field keeps what was typed: the change has only been proposed, and
   *  cancelling must not cost the user their typing. */
  it('emits the target name and keeps the field until the change is confirmed', async () => {
    let emitted: string | undefined;
    fixture.componentInstance.renameRequested.subscribe((into) => (emitted = into));
    await select('auth');
    await type('  identity ');

    fixture.debugElement.query(By.css('.tags-actions')).triggerEventHandler('submit', new Event('submit'));
    await fixture.whenStable();

    expect(emitted).toBe('identity');
    expect(fixture.nativeElement.querySelector('.tags-target').value).toBe('  identity ');
  });

  it('does not rename towards nothing', async () => {
    let emitted = 0;
    fixture.componentInstance.renameRequested.subscribe(() => (emitted += 1));
    await select('auth');
    await type('   ');

    fixture.debugElement.query(By.css('.tags-actions')).triggerEventHandler('submit', new Event('submit'));
    await fixture.whenStable();

    expect(emitted).toBe(0);
  });

  it('asks rather than acting, when a tag action is clicked', async () => {
    let emitted = 0;
    fixture.componentInstance.deleteRequested.subscribe(() => (emitted += 1));
    await select('auth');

    deleteButton().click();
    await fixture.whenStable();

    expect(emitted).toBe(1);
  });

  /**
   * In place of the actions, not beside them: the click that asked for this is the one
   * that would confirm it, and a second click landing on the same spot is the accident a
   * confirmation exists to stop.
   */
  it('replaces the actions with what the change would touch', async () => {
    await select('auth');
    await propose({ kind: 'delete', tags: ['auth'], into: '', notes: 4 });

    const confirmation = fixture.nativeElement.querySelector('[data-testid="tag-confirm"]');

    expect(confirmation.textContent).toContain('4');
    expect(fixture.nativeElement.querySelector('.tags-actions')).toBeNull();
  });

  it('says a merge cannot be undone, because it is the one that cannot', async () => {
    await select('auth', 'api');
    await propose({ kind: 'merge', tags: ['auth', 'api'], into: 'backend', notes: 5 });

    const confirmation = fixture.nativeElement.querySelector('[data-testid="tag-confirm"]');

    expect(confirmation.textContent).toContain('ne s’annule pas');
  });

  it('emits the confirmation, and the cancellation', async () => {
    let confirmed = 0;
    let cancelled = 0;
    fixture.componentInstance.confirmed.subscribe(() => (confirmed += 1));
    fixture.componentInstance.cancelled.subscribe(() => (cancelled += 1));
    await select('auth');
    await propose({ kind: 'rename', tags: ['auth'], into: 'identity', notes: 4 });

    fixture.nativeElement.querySelector('[data-testid="tag-confirm-apply"]').click();
    fixture.nativeElement.querySelector('[data-testid="tag-confirm-cancel"]').click();
    await fixture.whenStable();

    expect(confirmed).toBe(1);
    expect(cancelled).toBe(1);
  });

  it('shows an empty state rather than empty controls', async () => {
    fixture.componentRef.setInput('tags', []);
    await fixture.whenStable();

    expect(fixture.nativeElement.querySelector('.tags-state').textContent).toContain('Aucun tag');
    expect(fixture.nativeElement.querySelector('.tags-actions')).toBeNull();
  });
});
