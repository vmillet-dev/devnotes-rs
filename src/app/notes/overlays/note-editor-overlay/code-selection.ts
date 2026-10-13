import type { Span } from '@core/model/json.model';

/**
 * What the visualiser asks of the code beside it, whichever engine draws it: the textarea
 * today, CodeMirror once it lands. Offsets are UTF-16, as the spans are.
 */
export interface CodeSelection {
  /** Selects the span and scrolls it a third of the way down; takes the focus only when asked. */
  reveal(span: Span, focus: boolean): void;
}

export function textareaSelection(field: HTMLTextAreaElement): CodeSelection {
  return {
    reveal(span, focus) {
      if (focus) field.focus({ preventScroll: true });
      field.setSelectionRange(span.start, span.end);
      const line = field.value.slice(0, span.start).split('\n').length - 1;
      const height = Number.parseFloat(getComputedStyle(field).lineHeight) || 22;
      const scroller = field.closest<HTMLElement>('.code-pane, .overlay-body');
      if (scroller) scroller.scrollTop = Math.max(0, line * height - scroller.clientHeight / 3);
    },
  };
}
