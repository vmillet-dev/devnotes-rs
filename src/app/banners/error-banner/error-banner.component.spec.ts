import { ComponentFixture, TestBed } from '@angular/core/testing';
import { By } from '@angular/platform-browser';
import { beforeEach, describe, expect, it } from 'vitest';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { ErrorBannerComponent } from './error-banner.component';

describe('ErrorBannerComponent', () => {
  let fixture: ComponentFixture<ErrorBannerComponent>;
  let notifier: ErrorNotifier;

  beforeEach(() => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [ErrorBannerComponent],
      providers: [provideTranslocoTesting()],
    });
    fixture = TestBed.createComponent(ErrorBannerComponent);
    notifier = TestBed.inject(ErrorNotifier);
    fixture.autoDetectChanges();
  });

  it('renders nothing while there is no notice', () => {
    expect(fixture.debugElement.query(By.css('.error-banner'))).toBeNull();
  });

  it('renders the translated message and the technical detail', async () => {
    notifier.notify({ ref: { key: 'errors.noteSaveFailed' }, detail: 'disk full' });
    await fixture.whenStable();

    expect(fixture.nativeElement.querySelector('.error-text').textContent).toContain(
      'Impossible d’enregistrer la note',
    );
    expect(fixture.nativeElement.querySelector('.error-detail').textContent).toBe('disk full');
  });

  it('interpolates the notice parameters', async () => {
    notifier.notify({ ref: { key: 'errors.ipcFailed', params: { command: 'query_notes' } } });
    await fixture.whenStable();

    expect(fixture.nativeElement.querySelector('.error-text').textContent).toContain('query_notes');
  });

  it('announces itself as an alert so it is read without stealing focus', async () => {
    notifier.notify({ ref: { key: 'errors.unexpected' } });
    await fixture.whenStable();

    expect(fixture.nativeElement.querySelector('.error-banner').getAttribute('role')).toBe('alert');
  });

  it('clears the notice when dismissed', async () => {
    notifier.notify({ ref: { key: 'errors.unexpected' } });
    await fixture.whenStable();

    fixture.debugElement.query(By.css('.error-dismiss')).triggerEventHandler('click');
    await fixture.whenStable();

    expect(notifier.notice()).toBeNull();
    expect(fixture.debugElement.query(By.css('.error-banner'))).toBeNull();
  });
});
