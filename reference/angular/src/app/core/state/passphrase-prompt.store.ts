import { Injectable, signal } from '@angular/core';

/**
 * Sealing a file about to be written, or opening one about to be read: the first confirms the
 * phrase and may be declined, the second cannot be.
 */
export interface PassphraseRequest {
  readonly purpose: 'protect' | 'unlock';
  readonly fileName: string;
  /** The previous attempt was refused: said beside the field. */
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

  /** A phrase is being derived from: the prompt waits on screen, and cannot be answered twice. */
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
   * The prompt's only way back in. A phrase leaves it on screen, working — derivation takes
   * about a second, and a dialog vanishing and coming back on a typo would read as a fault.
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
