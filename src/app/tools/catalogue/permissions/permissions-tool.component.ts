import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { ToolsRepository } from '@core/data/tools.repository';
import { Class, ClassRights, ModeReading } from '@core/model/tool-answers.model';
import { liveResult } from '@core/services/tools/live-result';
import { Tool, ToolResult } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import { OutputRowComponent } from '@tools/ui/output-row/output-row.component';

type Read = Extract<ModeReading, { kind: 'read' }>;
type Field = 'octal' | 'symbolic';
type Right = 'read' | 'write' | 'execute';

const SHIFTS: Record<Class, number> = { owner: 6, group: 3, others: 0 };
const RIGHTS: Record<Right, number> = { read: 4, write: 2, execute: 1 };
const SPECIALS: Record<Class, number> = { owner: 0o4000, group: 0o2000, others: 0o1000 };

/** The boxes before anything is typed: every one unticked, ready to be ticked. */
const NO_RIGHTS: readonly ClassRights[] = (['owner', 'group', 'others'] as const).map((row) => ({
  class: row,
  read: false,
  write: false,
  execute: false,
  special: false,
}));

/** What was typed, in the field it was typed in: the other field shows Rust's reading of it. */
interface Typed {
  readonly field: Field;
  readonly text: string;
}

/** Three ways into one mode — octal, letters, boxes — each written back from Rust's answer. */
@Component({
  selector: 'app-permissions-tool',
  imports: [OutputRowComponent, TranslocoPipe],
  templateUrl: './permissions-tool.component.html',
  styleUrl: './permissions-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class PermissionsToolComponent implements Tool {
  private readonly repository = inject(ToolsRepository);
  private readonly transloco = inject(TranslocoService);

  protected readonly typed = toolState<Typed>('permissions.typed', { field: 'octal', text: '' });
  protected readonly umask = toolState('permissions.umask', '022');

  protected readonly rights: readonly Right[] = ['read', 'write', 'execute'];

  private readonly translation = toSignal(this.transloco.selectTranslation());

  protected readonly answer = liveResult(
    () =>
      this.typed().text.trim() === '' && this.umask().trim() === ''
        ? undefined
        : { mode: this.typed().text, umask: this.umask() },
    (request) => this.repository.describePermissions(request),
  );

  protected readonly read = computed<Read | null>(() => {
    const mode = this.answer.value()?.mode;
    return mode?.kind === 'read' ? mode : null;
  });

  protected readonly problem = computed(() => {
    const mode = this.answer.value()?.mode;
    return mode?.kind === 'refused' ? mode : null;
  });

  protected readonly umaskReading = computed(() => this.answer.value()?.umask ?? null);

  protected readonly grid = computed(() => this.read()?.mode.classes ?? NO_RIGHTS);

  /** "Propriétaire : lire, écrire, exécuter". */
  protected readonly words = computed(() => {
    this.translation();
    const t = (key: string) => this.transloco.translate<string>(key);
    return (this.read()?.mode.classes ?? []).map((rights: ClassRights) => {
      const granted = this.rights
        .filter((right) => rights[right])
        .map((right) => t(`tools.permissions.words.${right}`));
      return {
        class: rights.class,
        text: granted.length > 0 ? granted.join(', ') : t('tools.permissions.words.none'),
        special: rights.special,
      };
    });
  });

  readonly result = computed<ToolResult | null>(() => {
    this.translation();
    const mode = this.read()?.mode;
    if (!mode) return null;
    const file = this.transloco.translate<string>('tools.permissions.file');
    return {
      title: { key: 'tools.permissions.noteTitle', params: { octal: mode.octal } },
      kind: 'snippet',
      language: 'sh',
      content: [
        `chmod ${mode.octal} ${file}`,
        `chmod ${mode.chmodSymbolic} ${file}`,
        `# ${mode.symbolic}`,
      ].join('\n'),
    };
  });

  /** A field shows what was typed in it, or else Rust's writing of the mode. */
  protected shown(field: Field): string {
    const typed = this.typed();
    if (typed.field === field) return typed.text;
    return this.read()?.mode[field] ?? '';
  }

  clear(): void {
    this.typed.set({ field: 'octal', text: '' });
    this.umask.set('');
  }

  protected checked(rights: ClassRights, right: Right | 'special'): boolean {
    return rights[right];
  }

  /** One box flips one bit; the mode is written back as octal, which Rust reads again. */
  protected toggle(changed: Class, right: Right | 'special'): void {
    const bits = this.read()?.mode.bits ?? 0;
    const mask = right === 'special' ? SPECIALS[changed] : RIGHTS[right] << SHIFTS[changed];
    const next = bits ^ mask;
    this.typed.set({ field: 'octal', text: next.toString(8).padStart(next > 0o777 ? 4 : 3, '0') });
  }

  protected onField(field: Field, event: Event): void {
    this.typed.set({ field, text: (event.target as HTMLInputElement).value });
  }

  protected onUmask(event: Event): void {
    this.umask.set((event.target as HTMLInputElement).value);
  }
}
