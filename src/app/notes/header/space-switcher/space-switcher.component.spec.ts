import { ComponentFixture, TestBed } from '@angular/core/testing';
import { By } from '@angular/platform-browser';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { Space } from '@core/model/space.model';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { SpaceSwitcherComponent } from './space-switcher.component';

const SPACES: readonly Space[] = [
  { id: 'work', name: 'Work', pinned: false },
  { id: 'personal', name: 'Personal', pinned: false },
];

describe('SpaceSwitcherComponent', () => {
  let fixture: ComponentFixture<SpaceSwitcherComponent>;

  function trigger(): HTMLButtonElement {
    return fixture.nativeElement.querySelector('.space-switch');
  }

  /** Every menu entry, in DOM order: "all spaces", the spaces, then "new space". */
  function options(): HTMLButtonElement[] {
    return [...fixture.nativeElement.querySelectorAll('.space-option')];
  }

  function spaceOptions(): HTMLButtonElement[] {
    return options().slice(1, -1);
  }

  function createTrigger(): HTMLButtonElement {
    return fixture.nativeElement.querySelector('.space-create-trigger');
  }

  function nameInput(): HTMLInputElement | null {
    return fixture.nativeElement.querySelector('.space-create-input');
  }

  async function open(): Promise<void> {
    trigger().click();
    await fixture.whenStable();
  }

  async function pressKey(key: string): Promise<void> {
    (document.activeElement ?? document.body).dispatchEvent(
      new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true }),
    );
    await fixture.whenStable();
  }

  beforeEach(async () => {
    TestBed.configureTestingModule({
      imports: [SpaceSwitcherComponent],
      providers: [provideTranslocoTesting()],
    });
    fixture = TestBed.createComponent(SpaceSwitcherComponent);
    fixture.componentRef.setInput('spaces', SPACES);
    fixture.componentRef.setInput('activeSpace', SPACES[0]);
    // jsdom only tracks `document.activeElement` for attached elements.
    document.body.appendChild(fixture.nativeElement);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  afterEach(() => {
    fixture.nativeElement.remove();
  });

  it('shows the active space name and keeps the dropdown closed by default', () => {
    expect(trigger().textContent).toContain('Work');
    expect(fixture.debugElement.query(By.css('.space-dropdown'))).toBeNull();
  });

  it('labels the trigger "all spaces" when no space is selected', async () => {
    fixture.componentRef.setInput('activeSpace', null);
    await fixture.whenStable();

    expect(trigger().textContent).toContain('Tous les espaces');
  });

  it('opens the dropdown listing every space, framed by "all spaces" and "new space"', async () => {
    await open();

    expect(options().map((option) => option.textContent?.trim())).toEqual([
      'Tous les espaces',
      'Work',
      'Personal',
      '＋ Nouvel espace',
    ]);
  });

  it('marks the active space option', async () => {
    await open();

    expect(spaceOptions()[0].classList.contains('active')).toBe(true);
    expect(spaceOptions()[0].getAttribute('aria-checked')).toBe('true');
    expect(spaceOptions()[1].getAttribute('aria-checked')).toBe('false');
  });

  it('marks the "all spaces" option when no space is selected', async () => {
    fixture.componentRef.setInput('activeSpace', null);
    await open();

    expect(options()[0].getAttribute('aria-checked')).toBe('true');
  });

  it('emits spaceChanged and closes the dropdown when a space is selected', async () => {
    let emitted: string | null | undefined;
    fixture.componentInstance.spaceChanged.subscribe((id) => (emitted = id));

    await open();
    spaceOptions()[1].click();
    await fixture.whenStable();

    expect(emitted).toBe('personal');
    expect(fixture.debugElement.query(By.css('.space-dropdown'))).toBeNull();
  });

  it('emits null when "all spaces" is selected', async () => {
    let emitted: string | null | undefined = 'untouched';
    fixture.componentInstance.spaceChanged.subscribe((id) => (emitted = id));

    await open();
    options()[0].click();
    await fixture.whenStable();

    expect(emitted).toBeNull();
  });

  it('closes the dropdown when clicking outside the component', async () => {
    await open();
    expect(fixture.debugElement.query(By.css('.space-dropdown'))).not.toBeNull();

    const outsideClick = new MouseEvent('click');
    Object.defineProperty(outsideClick, 'target', { value: document.body });
    document.dispatchEvent(outsideClick);
    await fixture.whenStable();

    expect(fixture.debugElement.query(By.css('.space-dropdown'))).toBeNull();
  });

  it('does not close the dropdown when clicking inside the component', async () => {
    await open();

    const insideClick = new MouseEvent('click');
    Object.defineProperty(insideClick, 'target', { value: trigger() });
    document.dispatchEvent(insideClick);
    await fixture.whenStable();

    expect(fixture.debugElement.query(By.css('.space-dropdown'))).not.toBeNull();
  });

  describe('space creation', () => {
    async function openCreateForm(): Promise<void> {
      await open();
      createTrigger().click();
      await fixture.whenStable();
    }

    it('replaces the menu with a name field', async () => {
      await openCreateForm();

      expect(nameInput()).not.toBeNull();
      expect(options()).toHaveLength(0);
    });

    it('focuses the name field so it can be typed into right away', async () => {
      await openCreateForm();

      expect(document.activeElement).toBe(nameInput());
    });

    it('emits the raw name on submit and closes the dropdown', async () => {
      let emitted: string | undefined;
      fixture.componentInstance.spaceCreated.subscribe((name) => (emitted = name));
      await openCreateForm();

      const input = nameInput() as HTMLInputElement;
      input.value = 'Side project';
      input.form?.dispatchEvent(new Event('submit', { cancelable: true }));
      await fixture.whenStable();

      expect(emitted).toBe('Side project');
      expect(fixture.debugElement.query(By.css('.space-dropdown'))).toBeNull();
    });

    it('ignores a blank name instead of creating a nameless space', async () => {
      let emitted = false;
      fixture.componentInstance.spaceCreated.subscribe(() => (emitted = true));
      await openCreateForm();

      const input = nameInput() as HTMLInputElement;
      input.value = '   ';
      input.form?.dispatchEvent(new Event('submit', { cancelable: true }));
      await fixture.whenStable();

      expect(emitted).toBe(false);
      expect(nameInput()).not.toBeNull();
    });

    it('returns to the menu on Escape rather than closing the whole dropdown', async () => {
      await openCreateForm();

      await pressKey('Escape');

      expect(nameInput()).toBeNull();
      expect(options()).not.toHaveLength(0);
    });
  });

  describe('keyboard and assistive technology', () => {
    it('announces the expanded state of the trigger', async () => {
      expect(trigger().getAttribute('aria-expanded')).toBe('false');

      await open();

      expect(trigger().getAttribute('aria-expanded')).toBe('true');
    });

    it('moves focus to the first option on open, so the menu is reachable at all', async () => {
      await open();

      expect(document.activeElement).toBe(options()[0]);
    });

    it('moves through the options with the arrow keys, wrapping around', async () => {
      await open();
      const items = options();

      await pressKey('ArrowDown');
      expect(document.activeElement).toBe(items[1]);

      await pressKey('ArrowUp');
      expect(document.activeElement).toBe(items[0]);

      await pressKey('ArrowUp');
      expect(document.activeElement).toBe(items[items.length - 1]);
    });

    it('jumps to the first and last option with Home and End', async () => {
      await open();
      const items = options();

      await pressKey('End');
      expect(document.activeElement).toBe(items[items.length - 1]);

      await pressKey('Home');
      expect(document.activeElement).toBe(items[0]);
    });

    it('closes on Escape and returns focus to the trigger', async () => {
      await open();

      await pressKey('Escape');

      expect(fixture.debugElement.query(By.css('.space-dropdown'))).toBeNull();
      expect(document.activeElement).toBe(trigger());
    });

    it('returns focus to the trigger after picking an option', async () => {
      await open();

      spaceOptions()[1].click();
      await fixture.whenStable();

      expect(document.activeElement).toBe(trigger());
    });
  });

  describe('editing a space', () => {
    function editTriggers(): HTMLButtonElement[] {
      return [...fixture.nativeElement.querySelectorAll('.space-edit-trigger')];
    }

    function renameInput(): HTMLInputElement | null {
      return fixture.nativeElement.querySelector('[data-testid="space-rename-input"]');
    }

    function renameForm(): HTMLFormElement {
      return fixture.nativeElement.querySelector('.editor-panel form');
    }

    /** ⚠️ A menu now, not a `<select>`: the refuge is chosen, not typed. */
    function targetTrigger(): HTMLElement | null {
      return fixture.nativeElement.querySelector('[data-testid="choice-space-move-target"]');
    }

    async function targetOptions(): Promise<string[]> {
      targetTrigger()!.click();
      await fixture.whenStable();
      return [
        ...fixture.nativeElement.querySelectorAll(
          '[data-testid="choice-panel-space-move-target"] [data-option-id]',
        ),
      ].map((option) => (option as HTMLElement).getAttribute('data-option-id') ?? '');
    }

    function deleteButton(): HTMLButtonElement | null {
      return fixture.nativeElement.querySelector('[data-testid="space-delete"]');
    }

    /** Opens the dropdown if needed — after Escape it is already back on the menu. */
    async function edit(index: number): Promise<void> {
      if (!fixture.nativeElement.querySelector('.space-dropdown')) {
        await open();
      }
      editTriggers()[index].click();
      await fixture.whenStable();
    }

    it('offers one options button per space, none for "all spaces"', async () => {
      await open();

      expect(editTriggers()).toHaveLength(SPACES.length);
      expect(editTriggers()[0].getAttribute('aria-label')).toBe("Options de l'espace Work");
    });

    it('replaces the menu with the edit panel rather than nesting inside it', async () => {
      await edit(0);

      expect(fixture.debugElement.query(By.css('.space-menu'))).toBeNull();
      expect(renameInput()?.value).toBe('Work');
    });

    it('emits the new name on submit and closes', async () => {
      const renames: { id: string; name: string }[] = [];
      fixture.componentInstance.spaceRenamed.subscribe((event) => renames.push(event));
      await edit(0);

      renameInput()!.value = 'Boulot';
      renameForm().dispatchEvent(new Event('submit'));
      await fixture.whenStable();

      expect(renames).toEqual([{ id: 'work', name: 'Boulot' }]);
      expect(fixture.debugElement.query(By.css('.space-dropdown'))).toBeNull();
    });

    it('emits nothing for a blank name', async () => {
      const renames: unknown[] = [];
      fixture.componentInstance.spaceRenamed.subscribe(() => renames.push(true));
      await edit(0);

      renameInput()!.value = '   ';
      renameForm().dispatchEvent(new Event('submit'));
      await fixture.whenStable();

      expect(renames).toEqual([]);
    });

    it('offers every other space as a refuge for the notes', async () => {
      await edit(0);

      expect(await targetOptions()).toEqual(['personal']);
    });

    it('requires two clicks and emits the chosen refuge', async () => {
      const deletions: { id: string; targetSpaceId: string }[] = [];
      fixture.componentInstance.spaceDeleted.subscribe((event) => deletions.push(event));
      await edit(0);

      deleteButton()!.click();
      await fixture.whenStable();
      expect(deletions).toEqual([]);
      expect(deleteButton()!.textContent?.trim()).toBe('Confirmer ?');

      deleteButton()!.click();
      await fixture.whenStable();

      expect(deletions).toEqual([{ id: 'work', targetSpaceId: 'personal' }]);
    });

    it('refuses deletion outright when there is no other space', async () => {
      fixture.componentRef.setInput('spaces', [SPACES[0]]);
      await fixture.whenStable();

      await edit(0);

      expect(deleteButton()).toBeNull();
      expect(targetTrigger()).toBeNull();
      expect(fixture.nativeElement.querySelector('[data-testid="space-delete-blocked"]')).not.toBeNull();
    });

    it('returns to the menu on a first Escape, and closes on the second', async () => {
      await edit(0);

      await pressKey('Escape');
      expect(fixture.debugElement.query(By.css('.space-menu'))).not.toBeNull();

      await pressKey('Escape');
      expect(fixture.debugElement.query(By.css('.space-dropdown'))).toBeNull();
    });

    it('forgets a pending confirmation when the panel is left', async () => {
      await edit(0);
      deleteButton()!.click();
      await fixture.whenStable();

      await pressKey('Escape');
      await edit(0);

      expect(deleteButton()!.textContent?.trim()).toBe('Supprimer');
    });

    it('moves focus to the rename field so the panel is reachable by keyboard', async () => {
      await edit(0);

      expect(document.activeElement).toBe(renameInput());
    });
  });
});
