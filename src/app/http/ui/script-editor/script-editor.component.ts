import {
  ChangeDetectionStrategy,
  Component,
  computed,
  inject,
  input,
  linkedSignal,
  model,
  output,
} from '@angular/core';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import {
  HttpOrigin,
  InheritedScriptsRow,
  SCRIPT_PHASES,
  ScriptParts,
  ScriptPhase,
} from '@core/model/http.model';
import {
  SegmentedChoiceComponent,
  Segment,
} from '@shared/controls/segmented-choice/segmented-choice.component';
import { CodeEditorComponent } from '@notes/overlays/note-editor-overlay/code-editor/code-editor.component';
import { ScriptExample, ScriptExamplesMenuComponent } from './examples-menu/examples-menu.component';

/** A level of the order the scripts run in; `from` is `null` for the request itself. */
export interface ScriptLevelRow {
  readonly from: HttpOrigin | null;
  readonly name: string;
  readonly written: boolean;
}

const NO_LINE: ReadonlySet<number> = new Set();

function exampleCode(example: ScriptExample, testName: string): string {
  switch (example) {
    case 'readResponse':
      return 'const body = res.json();\nconsole.log(body);';
    case 'setVariable':
      return "env.set('name', 'value');";
    case 'addHeader':
      return "req.headers.set('X-Request-Id', crypto.randomUUID());";
    case 'sendRequest':
      return "const answer = await pm.sendRequest({ url: env.get('baseUrl'), method: 'GET' });\nconsole.log(answer.code);";
    case 'testStatus':
      return `test(${JSON.stringify(testName)}, () => {\n  expect(res.status).toBe(200);\n});`;
  }
}

/**
 * The two scripts of a request, a folder or a collection: before the send and after the answer,
 * in the snippets' code editor. A request's says where its own runs among those above it.
 */
@Component({
  selector: 'app-script-editor',
  imports: [CodeEditorComponent, ScriptExamplesMenuComponent, SegmentedChoiceComponent, TranslocoPipe],
  templateUrl: './script-editor.component.html',
  styleUrl: './script-editor.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ScriptEditorComponent {
  readonly scripts = input.required<ScriptParts>();
  /** What holds them: another one is another history, Ctrl+Z never reaches into it. */
  readonly owner = input.required<string>();
  readonly phase = model<ScriptPhase>('before');
  /** The line a failure named, marked until the next edit. */
  readonly line = input<number | null>(null);
  /** The folders and collection above a request; `null` for a folder's or a collection's own. */
  readonly above = input<readonly InheritedScriptsRow[] | null>(null);
  readonly requestName = input('');

  readonly scriptsChange = output<ScriptParts>();
  /** A level of the order clicked: its settings, on this script. */
  readonly levelOpened = output<HttpOrigin>();

  private readonly transloco = inject(TranslocoService);

  protected readonly segments: readonly Segment[] = SCRIPT_PHASES.map((id) => ({
    id,
    labelKey: `http.scripts.phases.${id}`,
  }));
  /** What a script reads, in the order the mockup said them. */
  protected readonly names = ['req', 'res', 'env', 'pm', "require('lodash')…"];
  protected readonly written = computed(() =>
    SCRIPT_PHASES.filter((phase) => this.scripts()[phase].trim() !== ''),
  );
  protected readonly source = computed(() => this.scripts()[this.phase()]);

  /** A fresh editor state for each document: the other phase's, another request's. */
  protected readonly session = linkedSignal<string, number>({
    source: () => `${this.owner()}\n${this.phase()}`,
    computation: (_, previous) => (previous?.value ?? 0) + 1,
  });

  private readonly edited = linkedSignal<string, boolean>({
    source: () => `${this.owner()}\n${this.phase()}\n${this.line()}`,
    computation: () => false,
  });
  protected readonly marked = computed<ReadonlySet<number>>(() => {
    const line = this.line();
    return line === null || this.edited() ? NO_LINE : new Set([line]);
  });

  /** Before the send from the collection down; after the answer, back up. */
  protected readonly order = computed<readonly ScriptLevelRow[] | null>(() => {
    const above = this.above();
    if (above === null) return null;
    const phase = this.phase();
    const levels: ScriptLevelRow[] = [
      ...above.map((level) => ({
        from: level.from,
        name: level.from.name,
        written: level.scripts[phase].trim() !== '',
      })),
      { from: null, name: this.requestName(), written: this.scripts()[phase].trim() !== '' },
    ];
    return phase === 'before' ? levels : levels.reverse();
  });

  protected choosePhase(id: string): void {
    this.phase.set(id as ScriptPhase);
  }

  protected onChanged(text: string): void {
    this.edited.set(true);
    this.scriptsChange.emit({ ...this.scripts(), [this.phase()]: text });
  }

  protected insert(example: ScriptExample): void {
    const code = exampleCode(example, this.transloco.translate('http.scripts.examples.testName'));
    const current = this.source().replace(/\s+$/, '');
    this.onChanged(current === '' ? `${code}\n` : `${current}\n\n${code}\n`);
  }
}
