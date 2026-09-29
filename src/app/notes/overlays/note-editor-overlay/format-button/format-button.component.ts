import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  Injector,
  afterNextRender,
  inject,
  input,
  output,
  viewChild,
} from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { PRETTIER_INDENTATIONS, PrettierSettings, clampPrintWidth } from '@core/services/format/format.model';
import { MenuTriggerDirective } from '@shared/directives/menu-trigger.directive';
import {
  Segment,
  SegmentedChoiceComponent,
} from '@shared/controls/segmented-choice/segmented-choice.component';

const INDENTATION_SEGMENTS: readonly Segment[] = PRETTIER_INDENTATIONS.map((id) => ({
  id,
  labelKey: `editor.format.indentations.${id}`,
}));

const QUOTE_SEGMENTS: readonly Segment[] = [
  { id: 'single', labelKey: 'editor.format.quotesSingle' },
  { id: 'double', labelKey: 'editor.format.quotesDouble' },
];

type Switch = 'semicolons' | 'trailingCommas' | 'formatOnSave';

/**
 * "Formater" and its chevron. The panel belongs to the library, not to the note: it says so
 * in its heading, and a change applies at once, with no button to confirm it.
 */
@Component({
  selector: 'app-format-button',
  imports: [TranslocoPipe, SegmentedChoiceComponent],
  hostDirectives: [MenuTriggerDirective],
  templateUrl: './format-button.component.html',
  styleUrl: './format-button.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class FormatButtonComponent {
  private readonly injector = inject(Injector);
  protected readonly menu = inject(MenuTriggerDirective);

  /** Whether Prettier formats the note's language: the button stays, and says why not. */
  readonly formattable = input.required<boolean>();
  readonly busy = input(false);
  readonly language = input.required<string>();
  readonly settings = input.required<PrettierSettings>();

  readonly formatRequested = output<void>();
  readonly settingsChanged = output<Partial<PrettierSettings>>();

  protected readonly indentations = INDENTATION_SEGMENTS;
  protected readonly quotes = QUOTE_SEGMENTS;
  protected readonly switches: readonly Switch[] = ['semicolons', 'trailingCommas', 'formatOnSave'];

  private readonly widthField = viewChild<ElementRef<HTMLInputElement>>('widthField');

  /** Into the panel, which is a dialog and not a menu: its first field takes the focus. */
  protected togglePanel(): void {
    this.menu.toggle();
    if (this.menu.open()) {
      afterNextRender(() => this.widthField()?.nativeElement.focus(), { injector: this.injector });
    }
  }

  protected onWidth(field: HTMLInputElement): void {
    const width = clampPrintWidth(field.valueAsNumber);
    if (width !== null && width !== this.settings().printWidth) {
      this.settingsChanged.emit({ printWidth: width });
    }
    field.value = String(width ?? this.settings().printWidth);
  }

  protected onIndentation(id: string): void {
    const indentation = PRETTIER_INDENTATIONS.find((each) => each === id);
    if (indentation) this.settingsChanged.emit({ indentation });
  }

  protected onQuotes(id: string): void {
    if (id === 'single' || id === 'double') this.settingsChanged.emit({ quotes: id });
  }

  protected onSwitch(key: Switch, field: HTMLInputElement): void {
    this.settingsChanged.emit({ [key]: field.checked });
  }
}
