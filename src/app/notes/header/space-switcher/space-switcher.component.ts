import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  computed,
  effect,
  inject,
  input,
  output,
  signal,
  viewChild,
} from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { Space } from '@core/model/space.model';
import { SpaceEditorComponent } from '@notes/header/space-editor/space-editor.component';
import { MenuPanelDirective } from '@shared/directives/menu-panel.directive';
import { MenuTriggerDirective } from '@shared/directives/menu-trigger.directive';

@Component({
  selector: 'app-space-switcher',
  imports: [TranslocoPipe, MenuPanelDirective, SpaceEditorComponent],
  hostDirectives: [MenuTriggerDirective],
  templateUrl: './space-switcher.component.html',
  styleUrl: './space-switcher.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SpaceSwitcherComponent {
  readonly spaces = input.required<readonly Space[]>();
  /** `null` = "all spaces", a choice in its own right and not a waiting state. */
  readonly activeSpace = input.required<Space | null>();

  readonly spaceChanged = output<string | null>();
  readonly spaceCreated = output<string>();

  protected readonly menu = inject(MenuTriggerDirective);

  protected readonly creating = signal(false);

  /** ⚠️ The panel replaces the menu: input fields inside a `role="menu"` are not valid ARIA. */
  protected readonly editing = signal<Space | null>(null);

  private readonly nameInput = viewChild<ElementRef<HTMLInputElement>>('nameInput');
  private readonly editor = viewChild(SpaceEditorComponent);

  /** A space cannot be its own refuge: the cascade would take the notes after the transfer. */
  protected readonly moveTargets = computed<readonly Space[]>(() => {
    const edited = this.editing();
    return edited ? this.spaces().filter((space) => space.id !== edited.id) : [];
  });

  constructor() {
    this.menu.handleEscape(() => this.onEscape());
    this.menu.closed.subscribe(() => this.resetPanels());

    effect(() => {
      if (!this.menu.open()) return;
      if (this.editing()) {
        this.editor()?.focusName();
      } else if (this.creating()) {
        this.nameInput()?.nativeElement.focus();
      }
    });
  }

  protected toggle(): void {
    this.menu.toggle();
    this.resetPanels();
  }

  protected select(space: Space | null): void {
    this.spaceChanged.emit(space?.id ?? null);
    this.menu.close();
  }

  protected startCreating(): void {
    this.creating.set(true);
  }

  protected startEditing(space: Space): void {
    this.editing.set(space);
  }

  /** `submit` and not `click`: the form then also answers Enter. */
  protected submitNewSpace(event: Event, name: string): void {
    event.preventDefault();
    if (!name.trim()) return;

    this.spaceCreated.emit(name);
    this.menu.close();
  }

  /** Escape closes the open panel first, then the menu itself. */
  private onEscape(): void {
    if (this.editing() || this.creating()) {
      this.resetPanels();
      return;
    }
    this.menu.close();
  }

  private resetPanels(): void {
    this.creating.set(false);
    this.editing.set(null);
  }
}
