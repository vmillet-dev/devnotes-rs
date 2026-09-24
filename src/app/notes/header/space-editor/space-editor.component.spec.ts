import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { Space } from '@core/model/space.model';
import { SpacesStore } from '@core/state/spaces.store';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { RecordingSpaceActions } from '@testing/recording-actions';
import { SpaceEditorComponent } from './space-editor.component';

const WORK: Space = { id: 'work', name: 'Work', pinned: false };
const PERSONAL: Space = { id: 'personal', name: 'Personal', pinned: false };

describe('SpaceEditorComponent', () => {
  let fixture: ComponentFixture<SpaceEditorComponent>;
  let actions: RecordingSpaceActions;
  let finished: number;

  function root(): HTMLElement {
    return fixture.nativeElement;
  }

  async function click(selector: string): Promise<void> {
    root().querySelector<HTMLElement>(selector)?.click();
    await fixture.whenStable();
  }

  beforeEach(async () => {
    actions = new RecordingSpaceActions();
    finished = 0;
    TestBed.configureTestingModule({
      imports: [SpaceEditorComponent],
      providers: [provideTranslocoTesting(), { provide: SpacesStore, useValue: actions }],
    });
    fixture = TestBed.createComponent(SpaceEditorComponent);
    fixture.componentRef.setInput('space', WORK);
    fixture.componentRef.setInput('moveTargets', [PERSONAL]);
    fixture.componentInstance.finished.subscribe(() => (finished += 1));
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  it('pins without closing', async () => {
    await click('[data-testid="space-pin"]');

    expect(actions.pinned).toEqual(['work']);
    expect(finished).toBe(0);
  });

  it('renames the space itself, then says it is done', async () => {
    const input = root().querySelector<HTMLInputElement>('[data-testid="space-rename-input"]');
    if (input) input.value = 'Boulot';
    root().querySelector('form')?.dispatchEvent(new Event('submit'));
    await fixture.whenStable();

    expect(actions.renamed).toEqual([{ id: 'work', name: 'Boulot' }]);
    expect(finished).toBe(1);
  });

  /** The cascade would take the notes with the space: a refuge is not optional. */
  it('deletes into the refuge on the second click only', async () => {
    await click('[data-testid="space-delete"]');
    expect(actions.deleted).toEqual([]);

    await click('[data-testid="space-delete"]');
    expect(actions.deleted).toEqual([{ id: 'work', targetSpaceId: 'personal' }]);
    expect(finished).toBe(1);
  });

  it('offers no deletion at all without another space to take the notes', async () => {
    fixture.componentRef.setInput('moveTargets', []);
    await fixture.whenStable();

    expect(root().querySelector('[data-testid="space-delete"]')).toBeNull();
    expect(root().querySelector('[data-testid="space-delete-blocked"]')).not.toBeNull();
  });
});
