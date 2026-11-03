import { ChangeDetectionStrategy, Component, computed, input, output, signal } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { AUTH_KINDS, AuthKind, InheritedParts, KeyPlace, RequestAuth } from '@core/model/http.model';
import { ChoiceMenuComponent, ChoiceOption } from '@shared/controls/choice-menu/choice-menu.component';
import { IconComponent } from '@shared/icon/icon.component';

const FRESH: Record<AuthKind, RequestAuth> = {
  inherit: { kind: 'inherit' },
  none: { kind: 'none' },
  basic: { kind: 'basic', username: '', password: '' },
  bearer: { kind: 'bearer', token: '' },
  apiKey: { kind: 'apiKey', name: '', value: '', place: 'header' },
};

/** A `{{variable}}` says nothing secret and is shown; anything else typed is a secret. */
const VARIABLE = /^\{\{[^{}]*\}\}$/;

/**
 * A request's auth, or a collection's or folder's. The secret half is masked until asked for, and
 * an inherited one says what it is and where it was set.
 */
@Component({
  selector: 'app-auth-editor',
  imports: [ChoiceMenuComponent, IconComponent, TranslocoPipe],
  templateUrl: './auth-editor.component.html',
  styleUrl: './auth-editor.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class AuthEditorComponent {
  readonly auth = input.required<RequestAuth>();
  /** What « Héritée » would take; `null` when there is nowhere yet to inherit from. */
  readonly inherited = input<InheritedParts | null>(null);
  /** A collection has nothing above it to inherit from. */
  readonly allowInherit = input(true);

  readonly authChange = output<RequestAuth>();

  protected readonly revealed = signal(false);

  protected readonly kinds = computed<readonly ChoiceOption[]>(() =>
    AUTH_KINDS.filter((kind) => kind !== 'inherit' || this.allowInherit()).map((kind) => ({
      id: kind,
      name: `http.auth.kinds.${kind}`,
      nameIsKey: true,
    })),
  );
  protected readonly places: readonly ChoiceOption[] = (['header', 'query'] as const).map((place) => ({
    id: place,
    name: `http.auth.places.${place}`,
    nameIsKey: true,
  }));

  protected onKind(kind: string | null): void {
    if (kind !== null && kind !== this.auth().kind) this.authChange.emit(FRESH[kind as AuthKind]);
  }

  protected onField(field: string, event: Event): void {
    this.authChange.emit({
      ...this.auth(),
      [field]: (event.target as HTMLInputElement).value,
    });
  }

  protected onPlace(place: string | null): void {
    const auth = this.auth();
    if (auth.kind === 'apiKey' && place !== null) this.authChange.emit({ ...auth, place: place as KeyPlace });
  }

  /** « Bearer {{accessToken}} », the secret masked unless it is a variable. */
  protected summary(auth: RequestAuth): string {
    const secret = (value: string) => (VARIABLE.test(value) ? value : '••••••');
    switch (auth.kind) {
      case 'basic':
        return `Basic ${auth.username}`;
      case 'bearer':
        return `Bearer ${secret(auth.token)}`;
      case 'apiKey':
        return `${auth.name}: ${secret(auth.value)}`;
      case 'inherit':
      case 'none':
        return '';
    }
  }
}
