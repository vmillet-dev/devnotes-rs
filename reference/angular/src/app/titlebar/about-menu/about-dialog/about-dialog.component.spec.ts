import { VERSION } from '@angular/core';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { By } from '@angular/platform-browser';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { APP_INFO, AppInfoService } from '@core/services/app-info/app-info.service';
import { FakeAppInfo } from '@testing/fake-app-info';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { AboutDialogComponent } from './about-dialog.component';

describe('AboutDialogComponent', () => {
  let fixture: ComponentFixture<AboutDialogComponent>;
  let appInfo: FakeAppInfo;

  beforeEach(() => {
    TestBed.resetTestingModule();
    appInfo = new FakeAppInfo();
    TestBed.configureTestingModule({
      imports: [AboutDialogComponent],
      providers: [{ provide: AppInfoService, useValue: appInfo }, provideTranslocoTesting()],
    });
    fixture = TestBed.createComponent(AboutDialogComponent);
    fixture.autoDetectChanges();
  });

  it('names itself to the dialog shell by its own title', () => {
    const panel = fixture.nativeElement.querySelector('.dialog-panel');

    expect(panel.getAttribute('aria-labelledby')).toBe('about-dialog-title');
    expect(fixture.nativeElement.querySelector('#about-dialog-title')).not.toBeNull();
  });

  it('shows the running version', () => {
    expect(fixture.nativeElement.querySelector('.about-version').textContent.trim()).toBe('v0.1.0');
  });

  it('falls back to a dash rather than inventing a version outside Tauri', async () => {
    appInfo.versionSignal.set(null);
    await fixture.whenStable();

    expect(fixture.nativeElement.querySelector('.about-version').textContent.trim()).toBe('—');
  });

  it('names the developer and the repository', () => {
    const facts = fixture.nativeElement.querySelector('.about-facts').textContent;
    expect(facts).toContain('Valentin MILLET');
    expect(facts).toContain('@vmillet-dev');
    expect(facts).toContain('github.com/vmillet-dev/devnotes-rs');
  });

  it('names the stack it was built with, read and not retyped', () => {
    const stack = fixture.nativeElement.querySelector('.about-stack').textContent as string;

    expect(stack).toContain(`Angular ${VERSION.full}`);
    expect(stack).toContain(`Rust ${APP_INFO.rustVersion}`);
    expect(stack).toContain('Tauri 2.0.0');
    // Generated or read from the framework: all a test can hold is that none is missing.
    expect(stack).not.toContain('undefined');
  });

  it('opens the repository through the seam rather than navigating the WebView', () => {
    fixture.debugElement.query(By.css('.about-repo')).triggerEventHandler('click');

    expect(appInfo.openedRepository).toBe(1);
    expect(fixture.nativeElement.querySelector('.about-repo').tagName).toBe('BUTTON');
  });

  it('emits on the close button', async () => {
    const closed = vi.fn();
    fixture.componentInstance.closed.subscribe(closed);

    fixture.debugElement.query(By.css('.about-close')).triggerEventHandler('click');
    await fixture.whenStable();

    expect(closed).toHaveBeenCalledTimes(1);
  });
});
