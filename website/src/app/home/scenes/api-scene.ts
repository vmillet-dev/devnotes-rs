import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';

import { field, part } from '../../core/browser';
import { CodeView, Line } from '../code-view';
import { ScenePlayer } from '../scene-player';

const ENDPOINT = 'https://api.staging.internal/v1/invoices';

/** The same request, in four of the twenty targets the client writes code for. */
const TARGETS = [
  { name: 'cURL', code: `curl '${ENDPOINT}?status=paid&limit=20' \\\n  -H 'Accept: application/json'` },
  {
    name: 'PowerShell',
    code: `Invoke-RestMethod -Method Get \`\n  -Uri '${ENDPOINT}?status=paid&limit=20' \`\n  -Headers @{ Accept = 'application/json' }`,
  },
  {
    name: 'fetch',
    code: `const res = await fetch(\n  '${ENDPOINT}?status=paid&limit=20',\n  { headers: { Accept: 'application/json' } },\n);`,
  },
  {
    name: 'HttpClient',
    code: `this.http.get<Invoice[]>(\n  '${ENDPOINT}',\n  { params: { status: 'paid', limit: 20 } },\n);`,
  },
] as const;

const ANSWER: readonly Line[] = [
  ['{'],
  ['  ', ['f', '"data"'], ': ['],
  ['    { ', ['f', '"id"'], ': ', ['s', '"in_1Q2w"'], ', ', ['f', '"total"'], ': ', ['n', '4900'], ' },'],
  ['    { ', ['f', '"id"'], ': ', ['s', '"in_1Q2x"'], ', ', ['f', '"total"'], ': ', ['n', '12000'], ' }'],
  ['  ],'],
  ['  ', ['f', '"has_more"'], ': ', ['k', 'true']],
  ['}'],
];

const MESSAGES = [
  { up: true, at: '10:42:01', text: '{"type":"subscribe","channel":"invoices"}' },
  { up: false, at: '10:42:01', text: '{"type":"subscribed"}' },
  { up: false, at: '10:42:09', text: '{"type":"invoice.paid","id":"in_1Q2w"}' },
  { up: false, at: '10:42:15', text: '{"type":"invoice.paid","id":"in_1Q2x"}' },
  { up: false, at: '10:42:31', text: '{"type":"invoice.created","id":"in_1Q2y"}' },
] as const;

/** A variable resolved from the environment, the send and its answer, the generated code, a WebSocket. */
@Component({
  selector: 'app-api-scene',
  imports: [CodeView],
  templateUrl: './api-scene.html',
  styleUrl: './api-scene.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: {
    '[attr.data-step]': 'step()',
    '[class.resolving]': 'resolving()',
    '[class.sending]': 'sending()',
    '[class.answered]': 'answered()',
  },
})
export class ApiScene {
  private readonly player = inject(ScenePlayer);
  protected readonly step = this.player.step;
  private readonly t = this.player.t;
  protected readonly field = field;
  protected readonly targets = TARGETS;
  protected readonly answer = ANSWER;
  protected readonly messages = MESSAGES;

  protected readonly staging = computed(() => this.step() > 0 || this.t() > 0.3);
  protected readonly resolving = computed(() => this.step() === 0 && this.t() > 0.45);
  protected readonly sending = computed(() => this.step() === 1 && this.t() > 0.08 && this.t() < 0.3);
  protected readonly answered = computed(() => this.step() === 1 && this.t() >= 0.3);
  protected readonly lines = computed(() =>
    this.step() === 1 ? Math.round(ANSWER.length * part(this.t(), 0.3, 0.8)) : ANSWER.length,
  );
  protected readonly target = computed(() =>
    this.step() === 2
      ? Math.min(TARGETS.length - 1, Math.floor(part(this.t(), 0.05, 0.95) * TARGETS.length))
      : 0,
  );
  protected readonly generated = computed(() => TARGETS[this.target()]?.code ?? '');
  protected readonly received = computed(() =>
    this.step() === 3 ? Math.round(MESSAGES.length * part(this.t(), 0.1, 0.9)) : 0,
  );
}
