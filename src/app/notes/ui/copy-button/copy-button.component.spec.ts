import { ComponentFixture, TestBed } from '@angular/core/testing';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { CLIPBOARD_ADAPTER, ClipboardAdapter } from '@core/services/clipboard/clipboard.service';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { CopyButtonComponent } from './copy-button.component';

describe('CopyButtonComponent', () => {
  let fixture: ComponentFixture<CopyButtonComponent>;
  let adapter: ClipboardAdapter;

  function button(): HTMLButtonElement {
    return fixture.nativeElement.querySelector('.copy-btn');
  }

  function status(): string {
    return fixture.nativeElement.querySelector('[role="status"]').textContent.trim();
  }

  async function build(writeText = vi.fn(async () => undefined)): Promise<void> {
    adapter = { readText: vi.fn(async () => ''), writeText };
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [CopyButtonComponent],
      providers: [provideTranslocoTesting(), { provide: CLIPBOARD_ADAPTER, useValue: adapter }],
    });
    fixture = TestBed.createComponent(CopyButtonComponent);
    fixture.componentRef.setInput('value', 'SELECT 1');
    fixture.autoDetectChanges();
    await fixture.whenStable();
  }

  beforeEach(async () => {
    // ⚠️ Only the timers: a bare useFakeTimers() also fakes requestAnimationFrame,
    // which the zoneless scheduler needs.
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    await build();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('sends the value to the clipboard', async () => {
    button().click();
    await fixture.whenStable();

    expect(adapter.writeText).toHaveBeenCalledWith('SELECT 1');
  });

  it('reads the text at the click when handed a function, and copies nothing on null', async () => {
    fixture.componentRef.setInput('value', async () => 'SELECT 2');
    button().click();
    await fixture.whenStable();
    expect(adapter.writeText).toHaveBeenCalledWith('SELECT 2');

    fixture.componentRef.setInput('value', async () => null);
    button().click();
    await fixture.whenStable();
    expect(adapter.writeText).toHaveBeenCalledTimes(1);
  });

  it('announces the copy, then stops announcing it', async () => {
    expect(status()).toBe('');

    button().click();
    await fixture.whenStable();
    expect(status()).not.toBe('');

    vi.advanceTimersByTime(2000);
    await fixture.whenStable();
    expect(status()).toBe('');
  });

  it('stays silent when the clipboard refuses', async () => {
    await build(vi.fn(async () => Promise.reject(new Error('no plugin'))));

    button().click();
    await fixture.whenStable();

    expect(status()).toBe('');
  });

  it('does not let the click reach an enclosing card', async () => {
    const onParentClick = vi.fn();
    fixture.nativeElement.addEventListener('click', onParentClick);

    button().dispatchEvent(new MouseEvent('click', { bubbles: true }));
    await fixture.whenStable();

    expect(onParentClick).not.toHaveBeenCalled();
  });

  it('renders the icon alone unless a label is asked for', async () => {
    expect(button().querySelectorAll('[aria-hidden="true"]')).toHaveLength(1);

    fixture.componentRef.setInput('showLabel', true);
    await fixture.whenStable();

    expect(button().querySelectorAll('[aria-hidden="true"]')).toHaveLength(2);
  });
});
