import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { TranslationRef } from '@core/services/i18n/translation-ref.model';
import { Refused } from '@core/services/shortcuts/shortcut-bindings.store';
import { SettingsDraftStore } from '@core/services/settings/settings-draft.store';
import {
  GLOBAL_ACTIONS,
  Rebindable,
  acceleratorFromEvent,
  canvasKeystrokeFromEvent,
} from '@core/services/shortcuts/shortcut.model';
import { CANVAS_ACTIONS } from '@notes/canvas-keyboard.directive';

/** A refusal belongs to the field that caused it: two rows, two independent messages. */
interface Refusal {
  readonly id: string;
  readonly ref: TranslationRef;
}

interface ShortcutSection {
  readonly id: string;
  readonly labelKey: string;
  readonly noteKey: string;
  readonly actions: readonly Rebindable[];
}

/**
 * Two sections because there are two storage paths, and the difference is not
 * cosmetic: a global key is taken from the whole machine and must carry a modifier,
 * where a canvas one answers only while the canvas has the keyboard.
 */
const SECTIONS: readonly ShortcutSection[] = [
  {
    id: 'global',
    labelKey: 'shortcuts.groups.global',
    noteKey: 'settings.shortcuts.globalNote',
    actions: GLOBAL_ACTIONS,
  },
  {
    id: 'canvas',
    labelKey: 'shortcuts.groups.canvas',
    noteKey: 'settings.shortcuts.canvasNote',
    actions: CANVAS_ACTIONS,
  },
];

/** Both paths at once: the check is worth nothing if it only looks at one table. */
const EVERY_ACTION = SECTIONS.flatMap((section) => section.actions);

@Component({
  selector: 'app-shortcuts-page',
  imports: [TranslocoPipe],
  templateUrl: './shortcuts-page.component.html',
  styleUrl: './shortcuts-page.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ShortcutsPageComponent {
  protected readonly draft = inject(SettingsDraftStore);
  private readonly transloco = inject(TranslocoService);

  protected readonly sections = SECTIONS;

  private readonly refusal = signal<Refusal | null>(null);

  protected readonly conflicts = computed(() =>
    this.draft.conflicts(EVERY_ACTION).map((conflict) => ({
      accelerator: conflict.accelerator,
      names: conflict.actions.map((action) => this.transloco.translate(action.labelKey)).join(', '),
    })),
  );

  protected refusalFor(id: string): TranslationRef | null {
    const refusal = this.refusal();

    return refusal?.id === id ? refusal.ref : null;
  }

  /**
   * The field listens for a keystroke rather than accepting text, which would let through
   * combinations the native side cannot read back. A modifier on its own is a
   * combination still being typed and goes back to the dialog — which is what leaves Tab
   * and Escape working inside the field.
   *
   * Captured through the reader its own half matches with: a global key is stored by
   * **position** because the native parser reads it back that way, a canvas one by the
   * **printed** character. Capturing both the same way puts one of the two on the wrong
   * key the moment the layout is not QWERTY.
   */
  protected onCapture(action: Rebindable, event: KeyboardEvent): void {
    const isGlobal = GLOBAL_ACTIONS.includes(action);
    const keystroke = isGlobal ? acceleratorFromEvent(event) : canvasKeystrokeFromEvent(event);
    if (keystroke === null) {
      // A bare key on a global row is a refusal worth saying out loud, but Tab and
      // Escape are not one — and the canvas reader is already what refuses those two.
      // Nothing is prevented here: the key was not taken, so it goes on working.
      if (isGlobal && canvasKeystrokeFromEvent(event) !== null) {
        this.refusal.set({ id: action.id, ref: { key: 'settings.shortcuts.needsModifier' } });
      }
      return;
    }

    event.preventDefault();

    const refused = this.draft.rebind(action, keystroke, EVERY_ACTION);
    this.refusal.set(refused === null ? null : this.explain(action, keystroke, refused));
  }

  protected reset(action: Rebindable): void {
    this.draft.resetBinding(action);
    this.refusal.set(null);
  }

  /** A refusal nobody can read is the same as a key that quietly did nothing. */
  private explain(action: Rebindable, keystroke: string, refused: Refused): Refusal {
    if (refused.kind === 'taken') {
      return {
        id: action.id,
        ref: {
          key: 'settings.shortcuts.taken',
          params: {
            accelerator: keystroke,
            action: this.transloco.translate(refused.by.labelKey),
          },
        },
      };
    }

    const isGlobal = GLOBAL_ACTIONS.includes(action);

    return {
      id: action.id,
      ref: { key: isGlobal ? 'settings.shortcuts.needsModifier' : 'settings.shortcuts.reserved' },
    };
  }
}
