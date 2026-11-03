import { ChangeDetectionStrategy, Component, OnInit, inject, output, signal } from '@angular/core';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { HttpRepository } from '@core/data/http.repository';
import { CookieDomain, JarCookie } from '@core/model/http.model';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { StatusNotifier } from '@core/services/notifications/status.service';
import { DialogComponent } from '@shared/layout/dialog/dialog.component';

/** The library's cookie jar: what answers set, by domain, to read and to throw away. */
@Component({
  selector: 'app-cookie-jar-dialog',
  imports: [DialogComponent, TranslocoPipe],
  templateUrl: './cookie-jar-dialog.component.html',
  styleUrl: './cookie-jar-dialog.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CookieJarDialogComponent implements OnInit {
  readonly closed = output<void>();

  private readonly repository = inject(HttpRepository);
  private readonly errors = inject(ErrorNotifier);
  private readonly status = inject(StatusNotifier);
  private readonly transloco = inject(TranslocoService);

  protected readonly domains = signal<readonly CookieDomain[] | null>(null);
  /** What « Vider le bocal » removes, asked of Rust before it runs. */
  protected readonly confirming = signal<number | null>(null);

  ngOnInit(): void {
    void this.load();
  }

  private async load(): Promise<void> {
    const domains = await this.errors.attempt('http.cookies.failed', () => this.repository.cookies());
    if (domains !== null) this.domains.set(domains);
  }

  protected expiry(cookie: JarCookie): string {
    if (cookie.expires === null) return this.transloco.translate('http.cookies.session');
    return new Intl.DateTimeFormat(this.transloco.activeLang(), {
      dateStyle: 'medium',
      timeStyle: 'short',
    }).format(new Date(cookie.expires));
  }

  protected async remove(id: string): Promise<void> {
    await this.run(() => this.repository.deleteCookie(id));
  }

  protected async removeDomain(domain: string): Promise<void> {
    await this.run(() => this.repository.deleteCookieDomain(domain));
  }

  protected async askClear(): Promise<void> {
    const count = await this.errors.attempt('http.cookies.failed', () => this.repository.countCookies());
    if (count !== null) this.confirming.set(count);
  }

  protected async clear(): Promise<void> {
    this.confirming.set(null);
    await this.run(() => this.repository.clearCookies());
  }

  private async run(removing: () => Promise<number>): Promise<void> {
    const removed = await this.errors.attempt('http.cookies.failed', removing);
    if (removed === null) return;
    this.status.notify({ key: 'http.cookies.removed', params: { count: removed } });
    await this.load();
  }
}
