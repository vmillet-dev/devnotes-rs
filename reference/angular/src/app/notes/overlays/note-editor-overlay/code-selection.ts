import type { Span } from '@core/model/json.model';

/** What the visualiser asks of the code beside it. Offsets are UTF-16, as the spans are. */
export interface CodeSelection {
  /** Selects the span and scrolls it a third of the way down; takes the focus only when asked. */
  reveal(span: Span, focus: boolean): void;
}
