import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { ToolsRepository } from '@core/data/tools.repository';
import { QrContent, QrCorrection, QrFormat, QrRequest, WifiSecurity } from '@core/model/tool-answers.model';
import { FileDialogService } from '@core/services/dialogs/file-dialog.service';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { StatusNotifier } from '@core/services/notifications/status.service';
import { liveResult } from '@core/services/tools/live-result';
import { Tool, ToolResult } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import {
  Segment,
  SegmentedChoiceComponent,
} from '@shared/controls/segmented-choice/segmented-choice.component';
import { IconComponent } from '@shared/icon/icon.component';
import { ResultRowComponent } from '@tools/ui/result-row/result-row.component';

type ContentKind = QrContent['kind'];

/** Every field of every form but the Wi-Fi password, which is a secret. */
interface QrForms {
  readonly text: string;
  readonly url: string;
  readonly ssid: string;
  readonly security: WifiSecurity;
  readonly hidden: boolean;
  readonly to: string;
  readonly subject: string;
  readonly body: string;
  readonly name: string;
  readonly phone: string;
  readonly email: string;
  readonly organisation: string;
}

type QrField = Exclude<keyof QrForms, 'security' | 'hidden'>;

const EMPTY: QrForms = {
  text: '',
  url: '',
  ssid: '',
  security: 'wpa',
  hidden: false,
  to: '',
  subject: '',
  body: '',
  name: '',
  phone: '',
  email: '',
  organisation: '',
};

const KINDS: readonly ContentKind[] = ['text', 'url', 'wifi', 'email', 'contact'];
const CORRECTIONS: readonly QrCorrection[] = ['low', 'medium', 'quartile', 'high'];
const SECURITIES: readonly WifiSecurity[] = ['wpa', 'wep', 'open'];
const DEFAULT_MARGIN = 4;
const MAX_MARGIN = 16;

const segments = (ids: readonly string[], prefix: string): readonly Segment[] =>
  ids.map((id) => ({ id, labelKey: `${prefix}.${id}` }));

/**
 * A payload from a form, its code drawn by Rust. ⚠️ The Wi-Fi password is a plain signal, never
 * a `toolState`: it dies with the tool, and a note keeping it is warned against.
 */
@Component({
  selector: 'app-qr-tool',
  imports: [IconComponent, ResultRowComponent, SegmentedChoiceComponent, TranslocoPipe],
  templateUrl: './qr-tool.component.html',
  styleUrl: './qr-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class QrToolComponent implements Tool {
  private readonly repository = inject(ToolsRepository);
  private readonly files = inject(FileDialogService);
  private readonly notifier = inject(ErrorNotifier);
  private readonly status = inject(StatusNotifier);

  protected readonly kind = toolState<ContentKind>('qr.kind', 'url');
  protected readonly forms = toolState<QrForms>('qr.forms', EMPTY);
  protected readonly correction = toolState<QrCorrection>('qr.correction', 'medium');
  protected readonly margin = toolState('qr.margin', DEFAULT_MARGIN);
  protected readonly password = signal('');

  protected readonly kinds = segments(KINDS, 'tools.qr.kinds');
  protected readonly corrections = CORRECTIONS.map((id) => ({ id, labelKey: `tools.qr.levels.${id}` }));
  protected readonly securities = segments(SECURITIES, 'tools.qr.securities');
  protected readonly maxMargin = MAX_MARGIN;
  protected readonly emailFields: readonly QrField[] = ['to', 'subject'];
  protected readonly contactFields: readonly QrField[] = ['name', 'phone', 'email', 'organisation'];

  private readonly content = computed<QrContent>(() => {
    const forms = this.forms();
    switch (this.kind()) {
      case 'text':
        return { kind: 'text', text: forms.text };
      case 'url':
        return { kind: 'url', url: forms.url };
      case 'wifi':
        return {
          kind: 'wifi',
          ssid: forms.ssid,
          password: this.password(),
          security: forms.security,
          hidden: forms.hidden,
        };
      case 'email':
        return { kind: 'email', to: forms.to, subject: forms.subject, body: forms.body };
      case 'contact':
        return {
          kind: 'contact',
          name: forms.name,
          phone: forms.phone,
          email: forms.email,
          organisation: forms.organisation,
        };
    }
  });

  protected readonly answer = liveResult(
    (): QrRequest => ({ content: this.content(), correction: this.correction(), margin: this.margin() }),
    (request) => this.repository.describeQrCode(request),
  );

  protected readonly code = computed(() => {
    const answer = this.answer.value();
    return answer?.kind === 'code' ? answer : null;
  });

  protected readonly tooLong = computed(() => {
    const answer = this.answer.value();
    return answer?.kind === 'tooLong' ? answer : null;
  });

  protected readonly viewBox = computed(() => {
    const side = this.code()?.side ?? 0;
    return `0 0 ${side} ${side}`;
  });

  readonly result = computed<ToolResult | null>(() => {
    const code = this.code();
    if (!code) return null;
    const content = this.answer.answered()?.content;
    const secret = content?.kind === 'wifi' && content.security !== 'open' && content.password !== '';
    return {
      title: { key: 'tools.qr.noteTitle' },
      kind: 'snippet',
      language: 'txt',
      content: code.payload,
      ...(secret ? { warning: { key: 'tools.qr.saveWarning' } } : {}),
    };
  });

  sample(): void {
    this.kind.set('url');
    this.forms.update((forms) => ({ ...forms, url: 'https://exemple.fr/doc' }));
  }

  clear(): void {
    this.forms.set(EMPTY);
    this.password.set('');
  }

  protected value(field: QrField | 'password'): string {
    return field === 'password' ? this.password() : this.forms()[field];
  }

  protected onField(field: QrField | 'password', event: Event): void {
    const value = (event.target as HTMLInputElement | HTMLTextAreaElement).value;
    if (field === 'password') {
      this.password.set(value);
    } else {
      this.forms.update((forms) => ({ ...forms, [field]: value }));
    }
  }

  protected onKind(id: string): void {
    this.kind.set(id as ContentKind);
  }

  protected onSecurity(id: string): void {
    this.forms.update((forms) => ({ ...forms, security: id as WifiSecurity }));
  }

  protected onHidden(event: Event): void {
    const hidden = (event.target as HTMLInputElement).checked;
    this.forms.update((forms) => ({ ...forms, hidden }));
  }

  protected onCorrection(id: string): void {
    this.correction.set(id as QrCorrection);
  }

  protected onMargin(event: Event): void {
    const margin = Number.parseInt((event.target as HTMLInputElement).value, 10);
    if (Number.isFinite(margin)) {
      this.margin.set(Math.min(Math.max(margin, 0), MAX_MARGIN));
    }
  }

  /** The code on screen, not the one being typed: the file is the preview. */
  protected async download(format: QrFormat): Promise<void> {
    const request = this.answer.answered();
    if (!request || !this.code()) return;
    const path = await this.files.chooseDestination(`qr-code.${format}`);
    if (path === null) return;

    const saved = await this.notifier.attempt('errors.toolFailed', () =>
      this.repository.saveQrCode(request, format, path),
    );
    if (saved?.kind === 'saved') {
      this.status.notify({ key: 'tools.qr.saved', params: { count: saved.bytes } });
    } else if (saved?.kind === 'failed') {
      this.notifier.notify({ ref: { key: `tools.files.${saved.problem}` } });
    }
  }

  protected async copyImage(): Promise<void> {
    const request = this.answer.answered();
    if (!request || !this.code()) return;
    const copied = await this.notifier.attempt('errors.toolFailed', () =>
      this.repository.copyQrCode(request),
    );
    if (copied) {
      this.status.notify({ key: 'tools.qr.copied' });
    }
  }
}
