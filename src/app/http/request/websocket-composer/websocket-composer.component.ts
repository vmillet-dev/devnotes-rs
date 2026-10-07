import { ChangeDetectionStrategy, Component, input, output, signal } from '@angular/core';
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

  protected onText(event: Event): void {
    this.text.set((event.target as HTMLTextAreaElement).value);
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
