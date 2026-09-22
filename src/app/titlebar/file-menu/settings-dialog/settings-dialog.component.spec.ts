import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { provideAppTesting } from '@testing/testing.providers';
import { SettingsDialogComponent } from './settings-dialog.component';

describe('SettingsDialogComponent', () => {
  let fixture: ComponentFixture<SettingsDialogComponent>;

  function railOptions(): HTMLButtonElement[] {
    return [...fixture.nativeElement.querySelectorAll('.settings-rail-option')];
  }

  function optionLabelled(label: string): HTMLButtonElement {
    const found = railOptions().find((option) => option.textContent?.includes(label));
    if (!found) throw new Error(`No rail option labelled "${label}"`);
    return found;
  }

  async function render(): Promise<void> {
    fixture = TestBed.createComponent(SettingsDialogComponent);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  }

  beforeEach(() => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [SettingsDialogComponent],
      providers: [provideAppTesting()],
    });
  });

  it('names itself to the dialog shell by its own title', async () => {
    await render();

    const panel = fixture.nativeElement.querySelector('.dialog-panel');

    expect(panel.getAttribute('aria-labelledby')).toBe('settings-dialog-title');
  });

  it('lists its pages in the order it declares them', async () => {
    await render();

    expect(railOptions().map((option) => option.textContent?.trim())).toEqual([
      'Général',
      'Raccourcis',
      'Sécurité',
      'Variables',
    ]);
  });

  it('opens on the general page, which is what the menu entry promised', async () => {
    await render();

    expect(optionLabelled('Général').getAttribute('aria-current')).toBe('page');
    expect(fixture.nativeElement.querySelector('app-variables-page')).toBeNull();
  });

  it('renders the chosen page through the outlet', async () => {
    await render();

    optionLabelled('Variables').click();
    await fixture.whenStable();

    expect(optionLabelled('Variables').getAttribute('aria-current')).toBe('page');
    expect(fixture.nativeElement.querySelector('app-variables-page')).not.toBeNull();
  });

  it('emits on the close button', async () => {
    await render();
    const closed = vi.fn();
    fixture.componentInstance.closed.subscribe(closed);

    fixture.nativeElement.querySelector('.settings-close').click();
    await fixture.whenStable();

    expect(closed).toHaveBeenCalledTimes(1);
  });
});
