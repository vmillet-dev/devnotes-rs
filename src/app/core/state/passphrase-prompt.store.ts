import { Injectable, signal } from '@angular/core';

/**
 * What the prompt is for: sealing a file about to be written, or opening one about to be
 * read. The two ask for different things — the first confirms the phrase and may be
 * declined, the second cannot be.
 */
export interface PassphraseRequest {
  readonly purpose: 'protect' | 'unlock';
  readonly fileName: string;
  /** The previous attempt was refused, which belongs beside the field and nowhere else. */
  readonly refused: boolean;
}

export type PassphraseAnswer =
  | { readonly kind: 'phrase'; readonly value: string }
  | { readonly kind: 'none' }
  | { readonly kind: 'cancelled' };

/**
 * The prompt drawn over the page while an export or an import waits for a phrase: the
 * operation asks, and the prompt answers.
 */
@Injectable({ providedIn: 'root' })
export class PassphrasePromptStore {
  private readonly _request = signal<PassphraseRequest | null>(null);
  private readonly _working = signal(false);

  /** What the prompt is asking for; `null` when it is not asking. */
  readonly request = this._request.asReadonly();

  /** A phrase has been given and is being derived from. The prompt waits rather than
   *  leaving the screen, and cannot be answered twice. */
  readonly working = this._working.asReadonly();

  private pending: ((answer: PassphraseAnswer) => void) | null = null;

  ask(request: PassphraseRequest): Promise<PassphraseAnswer> {
    return new Promise((resolve) => {
      this.pending = resolve;
      this._working.set(false);
      this._request.set(request);
    });
  }

  /**
   * The prompt's only way back in.
   *
   * ⚠️ A phrase leaves the prompt on screen, working: deriving the key takes about a
   * second, and a dialog that vanished and came back on a typo would read as a fault.
   * Anything else ends the asking there and then.
   */
  answer(answer: PassphraseAnswer): void {
    const resolve = this.pending;
    if (resolve === null) return;

    this.pending = null;
    if (answer.kind === 'phrase') {
      this._working.set(true);
    } else {
      this.close();
    }

    resolve(answer);
  }

  /** The prompt never outlives the operation it was opened for, failure included. */
  close(): void {
    this.pending = null;
    this._working.set(false);
    this._request.set(null);
  }
}
