import { ComponentFixture, TestBed } from '@angular/core/testing';
import { By } from '@angular/platform-browser';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { PassphraseAnswer, PassphrasePromptStore } from '@core/state/passphrase-prompt.store';
import { TransferStore } from '@core/state/transfer.store';
import { FakeFileDialog } from '@testing/fake-file-dialog';
import { FakeTransferRepository } from '@testing/fake-transfer-repository';
import { provideAppTesting } from '@testing/testing.providers';
import { PassphrasePromptComponent } from './passphrase-prompt.component';

describe('PassphrasePromptComponent', () => {
  let fixture: ComponentFixture<PassphrasePromptComponent>;
  let transfer: TransferStore;
  let prompt: PassphrasePromptStore;
  let repository: FakeTransferRepository;
  let dialog: FakeFileDialog;

  beforeEach(() => {
    TestBed.resetTestingModule();
    repository = new FakeTransferRepository();
    dialog = new FakeFileDialog();
    TestBed.configureTestingModule({
      imports: [PassphrasePromptComponent],
      providers: [provideAppTesting({ transferRepository: repository, fileDialog: dialog })],
    });
    transfer = TestBed.inject(TransferStore);
    prompt = TestBed.inject(PassphrasePromptStore);
    fixture = TestBed.createComponent(PassphrasePromptComponent);
    fixture.autoDetectChanges();
  });

  function element(hook: string): HTMLElement | null {
    return fixture.debugElement.query(By.css(`[data-testid="${hook}"]`))?.nativeElement ?? null;
  }

  function submitButton(): HTMLButtonElement {
    return element('passphrase-prompt-submit') as HTMLButtonElement;
  }

  async function type(hook: string, value: string): Promise<void> {
    const input = element(hook) as HTMLInputElement | null;
    if (input === null) throw new Error(`no field "${hook}"`);
    input.value = value;
    input.dispatchEvent(new Event('input'));
    await fixture.whenStable();
  }

  /**
   * The prompt is what an export or an import waits on, so a scenario starts one.
   * Wrapped rather than returned: an `async` helper handing back a promise would
   * adopt it, and wait for the very operation the prompt is blocking.
   */
  async function whileExporting(): Promise<{ done: Promise<void> }> {
    dialog.savePath = 'C:/out/library.devnotes';
    const done = transfer.export(null, new Date('2026-08-27T09:00:00Z'));
    await vi.waitFor(() => expect(element('passphrase-prompt')).not.toBeNull());
    return { done };
  }

  async function whileImporting(): Promise<{ done: Promise<boolean> }> {
    dialog.openPath = 'C:/in/library.devnotes';
    repository.fileIsProtected = true;
    repository.expectedPassphrase = 'a shared phrase';
    const done = transfer.import();
    await vi.waitFor(() => expect(element('passphrase-prompt')).not.toBeNull());
    return { done };
  }

  it('is not on screen while nothing is being asked for', () => {
    expect(element('passphrase-prompt')).toBeNull();
  });

  describe('protecting an export', () => {
    it('names the file it is about to write', async () => {
      const { done } = await whileExporting();

      expect(element('passphrase-prompt')?.textContent).toContain('library.devnotes');

      prompt.answer({ kind: 'cancelled' });
      await done;
    });

    /** Sold as protection, a four-character phrase is not protection. */
    it('refuses a phrase too short to be one', async () => {
      const { done } = await whileExporting();

      await type('passphrase-prompt-field', 'short');
      await type('passphrase-prompt-confirmation', 'short');

      expect(submitButton().disabled).toBe(true);
      expect(element('passphrase-prompt-problem')?.textContent?.trim()).not.toBe('');

      prompt.answer({ kind: 'cancelled' });
      await done;
    });

    it('refuses two entries that differ', async () => {
      const { done } = await whileExporting();

      await type('passphrase-prompt-field', 'a shared phrase');
      await type('passphrase-prompt-confirmation', 'a shared phrasr');

      expect(submitButton().disabled).toBe(true);

      prompt.answer({ kind: 'cancelled' });
      await done;
    });

    it('hands the phrase over once and clears the fields behind it', async () => {
      const { done } = await whileExporting();

      await type('passphrase-prompt-field', 'a shared phrase');
      await type('passphrase-prompt-confirmation', 'a shared phrase');
      submitButton().click();
      await done;
      await fixture.whenStable();

      expect(repository.exportedTo?.passphrase).toBe('a shared phrase');
      expect(element('passphrase-prompt')).toBeNull();
    });

    /** The escape has to stay, and has to be a deliberate second button: an export in
     *  the clear is the portable format, and the warning beside it is the point. */
    it('lets the file be written in the clear', async () => {
      const { done } = await whileExporting();

      (element('passphrase-prompt-plain') as HTMLButtonElement).click();
      await done;

      expect(repository.exportedTo?.passphrase).toBeNull();
    });
  });

  describe('opening a protected import', () => {
    it('asks for one field and no confirmation', async () => {
      const { done } = await whileImporting();

      expect(element('passphrase-prompt-confirmation')).toBeNull();
      expect(element('passphrase-prompt-plain')).toBeNull();

      prompt.answer({ kind: 'cancelled' });
      await done;
    });

    it('takes any phrase the file might have been sealed with', async () => {
      const { done } = await whileImporting();

      await type('passphrase-prompt-field', 'a shared phrase');
      submitButton().click();

      expect(await done).toBe(true);
      expect(repository.importedWith).toBe('a shared phrase');
    });

    it('says the phrase was refused, and empties the field for the next one', async () => {
      const { done } = await whileImporting();

      await type('passphrase-prompt-field', 'a typo');
      submitButton().click();
      await vi.waitFor(() => expect(element('passphrase-prompt-problem')?.textContent?.trim()).not.toBe(''));

      expect((element('passphrase-prompt-field') as HTMLInputElement).value).toBe('');

      prompt.answer({ kind: 'cancelled' });
      await done;
    });

    it('withdraws the refusal as soon as the field is touched again', async () => {
      const { done } = await whileImporting();

      await type('passphrase-prompt-field', 'a typo');
      submitButton().click();
      await vi.waitFor(() => expect(element('passphrase-prompt-problem')?.textContent?.trim()).not.toBe(''));

      await type('passphrase-prompt-field', 'another try');

      expect(element('passphrase-prompt-problem')?.textContent?.trim()).toBe('');

      prompt.answer({ kind: 'cancelled' });
      await done;
    });

    it('gives up rather than importing nothing silently', async () => {
      const { done } = await whileImporting();

      (element('passphrase-prompt-cancel') as HTMLButtonElement).click();

      expect(await done).toBe(false);
      expect(repository.importedWith).toBeNull();
    });
  });

  it('answers the store exactly once per request', async () => {
    const { done } = await whileExporting();
    const answers: PassphraseAnswer[] = [];
    const original = prompt.answer.bind(prompt);
    vi.spyOn(prompt, 'answer').mockImplementation((answer) => {
      answers.push(answer);
      original(answer);
    });

    (element('passphrase-prompt-cancel') as HTMLButtonElement).click();
    await done;

    expect(answers).toEqual([{ kind: 'cancelled' }]);
  });
});
