import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { FolderNamePromptComponent } from './folder-name-prompt.component';

describe('FolderNamePromptComponent', () => {
  let fixture: ComponentFixture<FolderNamePromptComponent>;
  let submitted: string[];
  let cancelled: number;

  function input(): HTMLInputElement {
    return fixture.nativeElement.querySelector('[data-testid="folder-name-input"]');
  }

  async function submit(name: string): Promise<void> {
    input().value = name;
    fixture.nativeElement.querySelector('form').dispatchEvent(new Event('submit'));
    await fixture.whenStable();
  }

  beforeEach(async () => {
    TestBed.configureTestingModule({
      imports: [FolderNamePromptComponent],
      providers: [provideTranslocoTesting()],
    });
    fixture = TestBed.createComponent(FolderNamePromptComponent);
    submitted = [];
    cancelled = 0;
    fixture.componentInstance.submitted.subscribe((name) => submitted.push(name));
    fixture.componentInstance.cancelled.subscribe(() => (cancelled += 1));
    document.body.appendChild(fixture.nativeElement);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  /** The band is already drawn: the only thing left to do is type. */
  it('is ready to type in as soon as it opens', () => {
    expect(document.activeElement).toBe(input());
  });

  it('answers the name typed', async () => {
    await submit('Reporting');

    expect(submitted).toEqual(['Reporting']);
  });

  it('answers nothing for a blank name', async () => {
    await submit('   ');

    expect(submitted).toEqual([]);
  });

  it('can be walked away from', async () => {
    fixture.nativeElement.querySelector('[data-testid="folder-name-cancel"]').click();
    await fixture.whenStable();

    expect(cancelled).toBe(1);
  });
});
