import { ChangelogRelease, ChangelogSpan } from '@core/services/app-info/changelog.service';

/** An entry as it usually is: one plain run. */
function plain(text: string): ChangelogSpan[] {
  return [{ kind: 'plain', text }];
}

/** The real one needs the Tauri bridge and the `opener` plugin, neither of which jsdom has. */
export class FakeChangelog {
  releases: readonly ChangelogRelease[] = [
    {
      version: '0.2.0',
      date: '2026-09-11',
      sections: [
        {
          title: 'Added',
          items: [
            [
              { kind: 'strong', text: 'Sample notes.' },
              { kind: 'plain', text: ' On first launch, in ' },
              { kind: 'code', text: 'their own space' },
              { kind: 'plain', text: '.' },
            ],
          ],
        },
      ],
    },
    {
      version: '0.1.0',
      date: null,
      sections: [{ title: '', items: [plain('Shipped at last.')] }],
    },
  ];

  /** When set, `load` rejects with it. */
  loadError: Error | null = null;

  openedReleases = 0;

  async load(): Promise<readonly ChangelogRelease[]> {
    if (this.loadError) throw this.loadError;
    return this.releases;
  }

  async openReleases(): Promise<void> {
    this.openedReleases += 1;
  }
}
