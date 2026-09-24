import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { UndoBarComponent } from './undo-bar.component';

describe('UndoBarComponent', () => {
  let fixture: ComponentFixture<UndoBarComponent>;

  beforeEach(async () => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [UndoBarComponent],
      providers: [provideTranslocoTesting()],
    });
    fixture = TestBed.createComponent(UndoBarComponent);
    fixture.componentRef.setInput('action', { kind: 'deletion', ids: ['a', 'b', 'c'], count: 3 });
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  it('says how many notes went to the trash', () => {
    expect(fixture.nativeElement.querySelector('.undo-text').textContent).toContain('3');
  });

  /** "3 notes deleted" and "3 notes moved" are not the same sentence. */
  it('says which action it is offering to put back', async () => {
    expect(fixture.nativeElement.querySelector('.undo-text').textContent).toContain('corbeille');

    fixture.componentRef.setInput('action', { kind: 'move', previous: [], count: 3 });
    await fixture.whenStable();

    expect(fixture.nativeElement.querySelector('.undo-text').textContent).toContain('déplacée');
  });

  it('announces itself as a status, not an alert', () => {
    expect(fixture.nativeElement.querySelector('.undo-bar').getAttribute('role')).toBe('status');
  });

  it('emits the undo request', async () => {
    let emitted = 0;
    fixture.componentInstance.undone.subscribe(() => (emitted += 1));

    fixture.nativeElement.querySelector('.undo-action').click();
    await fixture.whenStable();

    expect(emitted).toBe(1);
  });

  it('emits the dismissal', async () => {
    let emitted = 0;
    fixture.componentInstance.dismissed.subscribe(() => (emitted += 1));

    fixture.nativeElement.querySelector('.undo-dismiss').click();
    await fixture.whenStable();

    expect(emitted).toBe(1);
  });
});
