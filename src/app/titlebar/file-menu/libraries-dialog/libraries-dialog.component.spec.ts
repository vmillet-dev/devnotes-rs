import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { LibrariesStore } from '@core/state/libraries.store';
import { FakeAppWindow } from '@testing/fake-app-window';
import { FakeLibrariesRepository } from '@testing/fake-libraries-repository';
import { provideAppTesting } from '@testing/testing.providers';
import { LibrariesDialogComponent } from './libraries-dialog.component';

describe('LibrariesDialogComponent', () => {
  let fixture: ComponentFixture<LibrariesDialogComponent>;
  let appWindow: FakeAppWindow;
  let closed: number;

  function root(): HTMLElement {
    return fixture.nativeElement;
  }

  function within(library: string, testid: string): HTMLElement | null {
    return root().querySelector(
      `[data-library="${library}"] [data-testid="${testid}"], [data-testid="${testid}"][data-library="${library}"]`,
    );
  }

  async function click(element: HTMLElement | null): Promise<void> {
    element?.click();
    await fixture.whenStable();
  }

  async function open(names: readonly string[]): Promise<void> {
    appWindow = new FakeAppWindow();
    TestBed.configureTestingModule({
      imports: [LibrariesDialogComponent],
      providers: [provideAppTesting({ librariesRepository: new FakeLibrariesRepository(names), appWindow })],
    });
    await TestBed.inject(LibrariesStore).load();

    closed = 0;
    fixture = TestBed.createComponent(LibrariesDialogComponent);
    fixture.componentInstance.closed.subscribe(() => (closed += 1));
    document.body.appendChild(fixture.nativeElement);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  }

  describe('with two libraries', () => {
    beforeEach(() => open(['Notes', 'Archive']));

    it('lists them and says which one is open', () => {
      expect(root().querySelectorAll('[data-testid="library-row"]')).toHaveLength(2);
      expect(within('lib-0', 'library-open')).not.toBeNull();
      expect(within('lib-1', 'library-open')).toBeNull();
    });

    /** Deleting the files under a live connection takes the process down with them. */
    it('offers neither a switch nor a deletion on the open one', () => {
      expect(within('lib-0', 'library-switch')).toBeNull();
      expect(within('lib-0', 'library-delete')).toBeNull();
      expect(within('lib-1', 'library-delete')).not.toBeNull();
    });

    it('asks before deleting, and deletes only once confirmed', async () => {
      await click(within('lib-1', 'library-delete'));
      expect(root().querySelector('[data-testid="library-confirm"]')).not.toBeNull();

      await click(root().querySelector('[data-testid="library-confirm-delete"]'));

      await fixture.whenStable();
      expect(root().querySelectorAll('[data-testid="library-row"]')).toHaveLength(1);
    });

    it('keeps the library when the deletion is cancelled', async () => {
      await click(within('lib-1', 'library-delete'));
      await click(root().querySelector('[data-testid="library-cancel-delete"]'));

      expect(root().querySelector('[data-testid="library-confirm"]')).toBeNull();
      expect(root().querySelectorAll('[data-testid="library-row"]')).toHaveLength(2);
    });

    /** Opening another is a full teardown: the dialog does not stand over the gate. */
    it('switches by reloading the page, and closes', async () => {
      await click(within('lib-1', 'library-switch'));

      expect(appWindow.reloaded).toBe(1);
      expect(closed).toBe(1);
    });

    it('renames a library from its row', async () => {
      await click(within('lib-1', 'library-start-rename'));
      const input = root().querySelector<HTMLInputElement>('[data-testid="library-rename"]');
      if (input) input.value = 'Old notes';
      input?.dispatchEvent(new Event('input'));
      input?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter' }));
      await fixture.whenStable();

      await fixture.whenStable();
      expect(root().querySelector('[data-library="lib-1"]')?.textContent).toContain('Old notes');
    });
  });

  describe('with one library', () => {
    beforeEach(() => open(['Notes']));

    /** The last one cannot go: the gate would have nothing to offer. */
    it('offers no deletion at all', () => {
      expect(root().querySelector('[data-testid="library-delete"]')).toBeNull();
    });

    it('creates and opens a new one, then closes', async () => {
      const name = root().querySelector<HTMLInputElement>('[data-testid="library-new-name"]');
      const create = root().querySelector<HTMLButtonElement>('[data-testid="library-create"]');
      expect(create?.disabled).toBe(true);

      if (name) name.value = '  Work  ';
      name?.dispatchEvent(new Event('input'));
      await fixture.whenStable();
      await click(create);

      expect(appWindow.reloaded).toBe(1);
      expect(closed).toBe(1);
    });
  });
});
