import { ChangeDetectionStrategy, Component, computed, input, output, signal } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { SavedMessageRow, WebsocketParts } from '@core/model/http.model';

/** A saved message is named after its first line, cut: the name is a label, not a field. */
const NAME_LENGTH = 40;

/** The message to send on an open socket, and those kept to send again. */
@Component({
  selector: 'app-websocket-composer',
  imports: [TranslocoPipe],
  templateUrl: './websocket-composer.component.html',
  styleUrl: './websocket-composer.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class WebsocketComposerComponent {
  readonly websocket = input.required<WebsocketParts>();
  /** Sending needs an open socket; keeping a message does not. */
  readonly open = input.required<boolean>();
  readonly websocketChange = output<WebsocketParts>();
  readonly sent = output<string>();

  protected readonly text = signal('');
  /** `Ctrl+Entrée` was pressed on a socket that is not open: said, never connected unasked. */
  protected readonly refused = signal(false);
  /** What « not connected » is said by: gone as soon as the socket opens. */
  protected readonly notOpen = computed(() => this.refused() && !this.open());

  protected onText(event: Event): void {
    this.text.set((event.target as HTMLTextAreaElement).value);
  }

  /**
   * `Ctrl+Entrée` in the message sends the message. ⚠️ Stopped here: the page answers the same
   * key by connecting, whatever holds the focus, and did nothing of what was meant. Enter
   * alone stays a new line — a message is often several lines of JSON.
   */
  protected onKeydown(event: KeyboardEvent): void {
    if (event.key !== 'Enter' || !(event.ctrlKey || event.metaKey) || event.altKey || event.shiftKey) return;
    event.preventDefault();
    event.stopPropagation();
    this.refused.set(!this.open());
    this.send(this.text());
  }

  protected send(text: string): void {
    if (this.open() && text.trim() !== '') this.sent.emit(text);
  }

  protected keep(): void {
    const text = this.text();
    if (text.trim() === '') return;
    const name = text.trim().split('\n')[0]!.slice(0, NAME_LENGTH);
    this.change([...this.websocket().messages, { name, text }]);
  }

  protected remove(at: number): void {
    this.change(this.websocket().messages.filter((_, index) => index !== at));
  }

  protected load(message: SavedMessageRow): void {
    this.text.set(message.text);
  }

  private change(messages: readonly SavedMessageRow[]): void {
    this.websocketChange.emit({ ...this.websocket(), messages });
  }
}
