import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { SettingsStore } from '@core/services/settings/settings.store';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { GettingStartedDialogComponent } from './getting-started-dialog.component';

describe('GettingStartedDialogComponent', () => {
  let fixture: ComponentFixture<GettingStartedDialogComponent>;
  let settings: SettingsStore;

  const bodies = (): string =>
    Array.from(fixture.nativeElement.querySelectorAll('.guide-chapter-body'))
      .map((body) => (body as HTMLElement).textContent!)
      .join('\n');

  beforeEach(async () => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [GettingStartedDialogComponent],
      providers: [provideTranslocoTesting()],
    });
    settings = TestBed.inject(SettingsStore);
    fixture = TestBed.createComponent(GettingStartedDialogComponent);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  const title = (): string =>
    (fixture.nativeElement.querySelector('.guide-chapter-title') as HTMLElement).textContent!.trim();

  const dots = (): HTMLButtonElement[] =>
    Array.from(fixture.nativeElement.querySelectorAll('[data-testid="guide-dot"]'));

  async function press(testid: string): Promise<void> {
    (fixture.nativeElement.querySelector(`[data-testid="${testid}"]`) as HTMLButtonElement).click();
    await fixture.whenStable();
  }

  /** One chapter at a time. */
  it('shows one chapter, and walks to the next', async () => {
    expect(fixture.nativeElement.querySelectorAll('[data-testid="guide-chapter"]')).toHaveLength(1);
    expect(title()).toContain('notes');

    await press('guide-next');

    expect(title()).toContain('espaces');
  });

  it('draws the schematic of the chapter it is on', async () => {
    expect(fixture.nativeElement.querySelector('app-guide-figure svg')?.getAttribute('aria-hidden')).toBe(
      'true',
    );

    await press('guide-next');
    expect(fixture.nativeElement.querySelector('[data-chapter="spaces"]')).not.toBeNull();
  });

  /** A reader who knows which chapter they want must still be able to land on it. */
  it('jumps straight to a chapter from its dot', async () => {
    expect(dots()).toHaveLength(10);

    dots()[9].click();
    await fixture.whenStable();

    expect(title()).toContain('Entrer et sortir');
  });

  it('stops at both ends rather than wrapping round', async () => {
    expect(
      (fixture.nativeElement.querySelector('[data-testid="guide-previous"]') as HTMLButtonElement).disabled,
    ).toBe(true);

    dots()[9].click();
    await fixture.whenStable();

    expect(fixture.nativeElement.querySelector('[data-testid="guide-next"]')).toBeNull();
  });

  /** A walk you can only leave by finishing it is a wall with extra steps. */
  it('can be closed from any chapter, not only the last', async () => {
    let closed = 0;
    fixture.componentInstance.closed.subscribe(() => (closed += 1));

    await press('guide-close');

    expect(closed).toBe(1);
  });

  /** Reaching help from the thing it explains is what the empty canvas and board use. */
  it('opens at the chapter it was asked for', async () => {
    fixture.componentRef.setInput('startAt', 'folders');
    await fixture.whenStable();

    expect(title()).toContain('dossiers');
  });

  it('names the quick-paste key that is really bound', async () => {
    fixture.componentRef.setInput('startAt', 'palette');
    await fixture.whenStable();
    expect(bodies()).toContain('Ctrl+Alt+P');

    settings.setPaletteShortcut('Ctrl+Shift+K');
    await fixture.whenStable();

    expect(bodies()).toContain('Ctrl+Shift+K');
    expect(bodies()).not.toContain('Ctrl+Alt+P');
  });

  /** Transloco replaces an unknown `{{name}}` with the empty string, so a body that
   *  spells a key out silently loses it. Walked, because only one is on screen at a time. */
  it('leaves no interpolation unfilled, in any chapter', async () => {
    for (const dot of dots()) {
      dot.click();
      await fixture.whenStable();
      expect(bodies()).not.toContain('{{');
    }
  });
});
