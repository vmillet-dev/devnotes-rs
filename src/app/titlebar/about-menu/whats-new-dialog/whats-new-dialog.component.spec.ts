import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { AppInfoService } from '@core/services/app-info/app-info.service';
import { ChangelogService } from '@core/services/app-info/changelog.service';
import { FakeAppInfo } from '@testing/fake-app-info';
import { FakeChangelog } from '@testing/fake-changelog';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { WhatsNewDialogComponent } from './whats-new-dialog.component';

describe('WhatsNewDialogComponent', () => {
  let fixture: ComponentFixture<WhatsNewDialogComponent>;
  let changelog: FakeChangelog;
  let appInfo: FakeAppInfo;

  const text = (selector: string): string =>
    (fixture.nativeElement.querySelector(selector) as HTMLElement | null)?.textContent?.trim() ?? '';

  const items = (): string[] =>
    Array.from(fixture.nativeElement.querySelectorAll('.news-items li')).map((item) =>
      (item as HTMLElement).textContent!.trim(),
    );

  async function mount(): Promise<void> {
    TestBed.configureTestingModule({
      imports: [WhatsNewDialogComponent],
      providers: [
        { provide: ChangelogService, useValue: changelog },
        { provide: AppInfoService, useValue: appInfo },
        provideTranslocoTesting(),
      ],
    });
    fixture = TestBed.createComponent(WhatsNewDialogComponent);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  }

  beforeEach(() => {
    TestBed.resetTestingModule();
    changelog = new FakeChangelog();
    appInfo = new FakeAppInfo();
  });

  it('lists every release, newest first, with its date', async () => {
    await mount();

    const versions = Array.from(fixture.nativeElement.querySelectorAll('.news-version')).map(
      (version) => (version as HTMLElement).textContent,
    );
    expect(versions).toEqual(['0.2.0', '0.1.0']);
    expect(text('.news-date')).toBe('2026-09-11');
    expect(items()).toEqual(['Sample notes. On first launch, in their own space.', 'Shipped at last.']);
  });

  /**
   * ⚠️ The hand-written sections of `CHANGELOG.md` open on `**Todo-list notes.**` and
   * carry backticks, and all of it used to land on screen as punctuation. Rendered as
   * elements over a switch, never through `innerHTML`.
   */
  it('draws the runs an entry was cut into rather than their markers', async () => {
    await mount();

    const entry = fixture.nativeElement.querySelector('.news-items li') as HTMLElement;

    expect(entry.querySelector('strong')?.textContent).toBe('Sample notes.');
    expect(entry.querySelector('code')?.textContent).toBe('their own space');
    expect(entry.textContent).not.toContain('**');
  });

  it('marks the release the running binary is', async () => {
    appInfo.versionSignal.set('0.2.0');
    await mount();

    const heads = Array.from<HTMLElement>(fixture.nativeElement.querySelectorAll('.news-release-head'));
    expect(heads[0].querySelector('.news-installed')).not.toBeNull();
    expect(heads[1].querySelector('.news-installed')).toBeNull();
  });

  it('drops the heading of a release that lists its entries without a category', async () => {
    await mount();

    expect(fixture.nativeElement.querySelectorAll('.news-section-title')).toHaveLength(1);
  });

  it('says the changelog is unavailable rather than showing an empty one', async () => {
    changelog.loadError = new Error('no bridge');
    await mount();

    expect(text('.news-state')).toContain('indisponible');
    expect(fixture.nativeElement.querySelectorAll('.news-release')).toHaveLength(0);
  });

  it('says so when the file describes no release at all', async () => {
    changelog.releases = [];
    await mount();

    expect(text('.news-state')).toContain('Aucune version');
  });

  it('opens the releases page outside the WebView', async () => {
    await mount();

    (fixture.nativeElement.querySelector('.news-link') as HTMLButtonElement).click();
    await fixture.whenStable();

    expect(changelog.openedReleases).toBe(1);
  });

  it('closes from its button', async () => {
    await mount();
    let closed = 0;
    fixture.componentInstance.closed.subscribe(() => (closed += 1));

    (fixture.nativeElement.querySelector('.news-close') as HTMLButtonElement).click();
    await fixture.whenStable();

    expect(closed).toBe(1);
  });
});
