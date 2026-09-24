import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { Folder } from '@core/model/folder.model';
import { FoldersStore } from '@core/state/folders.store';
import { RecordingFolderActions } from '@testing/recording-actions';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { FolderBreadcrumbComponent } from './folder-breadcrumb.component';

const PERF: Folder = {
  id: 'perf',
  spaceId: 'sql',
  name: 'Perf',
  colour: 'amber',
  createdAt: new Date('2026-01-01T10:00:00Z'),
};

describe('FolderBreadcrumbComponent', () => {
  let fixture: ComponentFixture<FolderBreadcrumbComponent>;
  let actions: RecordingFolderActions;

  function root(): HTMLElement {
    return fixture.nativeElement;
  }

  async function click(selector: string): Promise<void> {
    root().querySelector<HTMLElement>(selector)?.click();
    await fixture.whenStable();
  }

  beforeEach(async () => {
    TestBed.resetTestingModule();
    actions = new RecordingFolderActions();
    TestBed.configureTestingModule({
      imports: [FolderBreadcrumbComponent],
      providers: [provideTranslocoTesting(), { provide: FoldersStore, useValue: actions }],
    });
    fixture = TestBed.createComponent(FolderBreadcrumbComponent);
    fixture.componentRef.setInput('folder', PERF);
    fixture.componentRef.setInput('spaceName', 'SQL');
    document.body.appendChild(fixture.nativeElement);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  it('reads space then folder, with the folder colour on its swatch', () => {
    expect(root().querySelector('[data-testid="folder-breadcrumb-back"]')?.textContent).toContain('SQL');
    expect(root().querySelector('[data-testid="folder-breadcrumb-name"]')?.textContent).toContain('Perf');
    expect(root().querySelector('.crumb-swatch')?.className).toContain('is-amber');
  });

  /** There is one place to go from inside a folder, and it is back. */
  it('asks to leave when the space is clicked', async () => {
    let left = 0;
    fixture.componentInstance.closed.subscribe(() => (left += 1));

    await click('[data-testid="folder-breadcrumb-back"]');

    expect(left).toBe(1);
  });

  it('names "all spaces" when no space is chosen', async () => {
    fixture.componentRef.setInput('spaceName', null);
    await fixture.whenStable();

    expect(root().querySelector('[data-testid="folder-breadcrumb-back"]')?.textContent).toContain(
      'Tous les espaces',
    );
  });

  it('keeps the folder actions one click away, and closed until asked', async () => {
    expect(root().querySelector('[data-testid="folder-breadcrumb-panel"]')).toBeNull();

    await click('[data-testid="folder-breadcrumb-menu"]');

    expect(root().querySelector('[data-testid="folder-rename-input"]')).not.toBeNull();
    expect(root().querySelectorAll('[data-testid="folder-colour"]')).toHaveLength(5);
  });

  it('renames from the panel and closes it', async () => {
    await click('[data-testid="folder-breadcrumb-menu"]');
    const input = root().querySelector<HTMLInputElement>('[data-testid="folder-rename-input"]');
    if (input) input.value = 'Performance';
    root().querySelector('form')?.dispatchEvent(new Event('submit'));
    await fixture.whenStable();

    expect(actions.renamed).toEqual([{ id: 'perf', name: 'Performance' }]);
    expect(root().querySelector('[data-testid="folder-breadcrumb-panel"]')).toBeNull();
  });

  /** Two steps: the WebView blocks on a native `confirm()`. */
  it('asks once before deleting, then goes back with the notes left standing', async () => {
    await click('[data-testid="folder-breadcrumb-menu"]');
    expect(root().querySelector('.editor-note')?.textContent).toContain('Les notes restent');

    await click('[data-testid="folder-delete"]');
    expect(actions.deleted).toEqual([]);

    await click('[data-testid="folder-delete"]');
    expect(actions.deleted).toEqual(['perf']);
  });
});
