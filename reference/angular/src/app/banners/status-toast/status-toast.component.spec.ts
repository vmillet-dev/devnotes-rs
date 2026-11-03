import { ComponentFixture, TestBed } from '@angular/core/testing';
import { By } from '@angular/platform-browser';
import { beforeEach, describe, expect, it } from 'vitest';
import { StatusNotifier } from '@core/services/notifications/status.service';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { StatusToastComponent } from './status-toast.component';

describe('StatusToastComponent', () => {
  let fixture: ComponentFixture<StatusToastComponent>;
  let notifier: StatusNotifier;

  beforeEach(() => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [StatusToastComponent],
      providers: [provideTranslocoTesting()],
    });
    fixture = TestBed.createComponent(StatusToastComponent);
    notifier = TestBed.inject(StatusNotifier);
    fixture.autoDetectChanges();
  });

  it('renders nothing while there is nothing to acknowledge', () => {
    expect(fixture.debugElement.query(By.css('.status-toast'))).toBeNull();
  });

  it('renders the report with its parameters interpolated', async () => {
    notifier.notify({ key: 'file.exported', params: { notes: '3', path: 'devnotes.json' } });
    await fixture.whenStable();

    const text = fixture.nativeElement.querySelector('.status-text').textContent;
    expect(text).toContain('3 notes exportées');
    expect(text).toContain('devnotes.json');
  });

  it('reports an operation that changed nothing, which failure looks just like', async () => {
    notifier.notify({ key: 'file.importedNothing', params: { skipped: '12' } });
    await fixture.whenStable();

    expect(fixture.nativeElement.querySelector('.status-text').textContent).toContain('12');
  });

  it('announces itself as a status, not an alert', async () => {
    notifier.notify({ key: 'file.exported', params: { notes: '1', path: 'devnotes.json' } });
    await fixture.whenStable();

    expect(fixture.nativeElement.querySelector('.status-toast').getAttribute('role')).toBe('status');
  });

  it('clears the report when dismissed', async () => {
    notifier.notify({ key: 'file.exported', params: { notes: '1', path: 'devnotes.json' } });
    await fixture.whenStable();

    fixture.debugElement.query(By.css('.status-dismiss')).triggerEventHandler('click');
    await fixture.whenStable();

    expect(notifier.status()).toBeNull();
    expect(fixture.debugElement.query(By.css('.status-toast'))).toBeNull();
  });
});
