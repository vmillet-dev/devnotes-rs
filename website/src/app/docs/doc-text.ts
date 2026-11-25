import { ChangeDetectionStrategy, Component, computed, input } from '@angular/core';
import { RouterLink } from '@angular/router';

import { Inline, Text } from './doc.model';

/** One shape for every kind, so the template reads its fields without narrowing. */
interface Run {
  readonly kind: 'text' | 'code' | 'keys' | 'strong' | 'ui' | 'route' | 'href';
  readonly text: string;
  readonly keys: readonly string[];
  readonly to: string;
}

function run(inline: Inline): Run {
  const plain = { text: '', keys: [], to: '' };
  if (typeof inline === 'string') return { ...plain, kind: 'text', text: inline };
  if ('code' in inline) return { ...plain, kind: 'code', text: inline.code };
  if ('keys' in inline) return { ...plain, kind: 'keys', keys: inline.keys };
  if ('strong' in inline) return { ...plain, kind: 'strong', text: inline.strong };
  if ('ui' in inline) return { ...plain, kind: 'ui', text: inline.ui };
  return { ...plain, kind: inline.to.startsWith('/') ? 'route' : 'href', text: inline.link, to: inline.to };
}

/**
 * A run of text with its inline marks. Every run is an element: the template compiler drops
 * the whitespace between two, so each space the sentence needs lives inside a run.
 */
@Component({
  selector: 'app-doc-text',
  imports: [RouterLink],
  template: `
    @for (part of runs(); track $index) {
      @switch (part.kind) {
        @case ('text') {
          <span>{{ part.text }}</span>
        }
        @case ('code') {
          <code>{{ part.text }}</code>
        }
        @case ('keys') {
          <span class="keys">
            @for (key of part.keys; track $index) {
              <kbd>{{ key }}</kbd>
            }
          </span>
        }
        @case ('strong') {
          <strong>{{ part.text }}</strong>
        }
        @case ('ui') {
          <b class="ui">{{ part.text }}</b>
        }
        @case ('route') {
          <a [routerLink]="part.to">{{ part.text }}</a>
        }
        @case ('href') {
          <a [href]="part.to">{{ part.text }}</a>
        }
      }
    }
  `,
  styles: `
    code {
      font-size: 0.88em;
      color: var(--amber-text);
      overflow-wrap: anywhere;
    }

    .ui {
      font-weight: 620;
      color: var(--text-0);
    }

    a {
      color: var(--amber-text);
      text-underline-offset: 3px;
    }
  `,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class DocText {
  readonly text = input.required<Text>();

  protected readonly runs = computed(() => this.text().map(run));
}
