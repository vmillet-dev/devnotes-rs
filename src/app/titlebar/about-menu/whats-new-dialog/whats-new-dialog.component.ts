import { ChangeDetectionStrategy, Component, computed, inject, output, resource } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { AppInfoService } from '@core/services/app-info/app-info.service';
import { ChangelogRelease, ChangelogService, RELEASES_URL } from '@core/services/app-info/changelog.service';
import { DialogComponent } from '@shared/layout/dialog/dialog.component';

const RELEASES_LABEL = RELEASES_URL.replace(/^https:\/\//, '');

/**
 * Not rendered from Markdown: the Rust side returns releases, categories and entries
 * already separated, so there is no renderer to pull in and no `innerHTML` to sanitize.
 */
@Component({
  selector: 'app-whats-new-dialog',
  imports: [DialogComponent, TranslocoPipe],
  templateUrl: './whats-new-dialog.component.html',
  styleUrl: './whats-new-dialog.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class WhatsNewDialogComponent {
  private readonly changelog = inject(ChangelogService);
  private readonly appInfo = inject(AppInfoService);

  readonly closed = output<void>();

  protected readonly releasesLabel = RELEASES_LABEL;

  private readonly releasesResource = resource({
    loader: () => this.changelog.load(),
    defaultValue: [] as readonly ChangelogRelease[],
  });

  /** Read behind `hasValue()`: `value()` throws while the resource is in error. */
  protected readonly releases = computed<readonly ChangelogRelease[]>(() =>
    this.releasesResource.hasValue() ? this.releasesResource.value() : [],
  );

  protected readonly isLoading = this.releasesResource.isLoading;

  protected readonly hasFailed = computed(() => this.releasesResource.error() !== undefined);

  protected readonly version = this.appInfo.version;

  /** Marks the release the running binary is: the rest is history or ahead of it. */
  protected isInstalled(release: ChangelogRelease): boolean {
    return release.version === this.version();
  }

  protected openReleases(): void {
    void this.changelog.openReleases();
  }
}
