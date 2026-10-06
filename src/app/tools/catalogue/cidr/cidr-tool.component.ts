import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { ToolsRepository } from '@core/data/tools.repository';
import {
  CidrBlock,
  CidrPlan,
  CidrSummary,
  IpFamily,
  MaskRow,
  ReverseDns,
} from '@core/model/tool-answers.model';
import { liveResult } from '@core/services/tools/live-result';
import { Tool, ToolResult } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import {
  Segment,
  SegmentedChoiceComponent,
} from '@shared/controls/segmented-choice/segmented-choice.component';
import { CopyValueComponent } from '@tools/ui/copy-value/copy-value.component';
import { ResultRowComponent } from '@tools/ui/result-row/result-row.component';

const AUTO = 'auto';
const SUPERSCRIPTS = '⁰¹²³⁴⁵⁶⁷⁸⁹';

const SAMPLE_PLAN = ['Bureaux 500', 'Wi-Fi invités 120', 'Serveurs 60', 'Imprimantes 12', 'Lien WAN 2'];
const SAMPLE_LIST = [
  '10.24.8.0/24',
  '10.24.9.0/24',
  '10.24.10.0/23',
  '10.24.12.0/24',
  '10.24.13.5 - 10.24.13.20',
];

/** The card's rows, in its order; IPv6 has neither mask nor broadcast. */
const ROWS = ['network', 'mask', 'inverseMask', 'first', 'last', 'broadcast'] as const;
type RowId = (typeof ROWS)[number];

interface Row {
  readonly id: RowId;
  readonly value: string;
}

interface Written {
  readonly id: 'integer' | 'expanded' | 'reverse';
  readonly value: string;
  /** A sentence about several zones is read, not pasted. */
  readonly copyable: boolean;
}

/** A block read and described by Rust; the page groups its digits in the reader's language. */
@Component({
  selector: 'app-cidr-tool',
  imports: [CopyValueComponent, ResultRowComponent, SegmentedChoiceComponent, TranslocoPipe],
  templateUrl: './cidr-tool.component.html',
  styleUrl: './cidr-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CidrToolComponent implements Tool {
  private readonly repository = inject(ToolsRepository);
  private readonly transloco = inject(TranslocoService);

  private readonly lang = toSignal(this.transloco.langChanges$, {
    initialValue: this.transloco.getActiveLang(),
  });

  protected readonly network = toolState('cidr.network', '');
  protected readonly family = toolState<IpFamily | null>('cidr.family', null);
  protected readonly member = toolState('cidr.member', '');
  protected readonly split = toolState<number | null>('cidr.split', null);
  protected readonly plan = toolState('cidr.plan', '');
  protected readonly list = toolState('cidr.list', '');

  protected readonly families: readonly Segment[] = [AUTO, 'v4', 'v6'].map((id) => ({
    id,
    labelKey: `tools.cidr.families.${id}`,
  }));

  protected readonly answer = liveResult(
    () => ({
      network: this.network(),
      family: this.family(),
      member: this.member(),
      split: this.split(),
      plan: this.plan(),
      list: this.list(),
    }),
    (request) => this.repository.describeCidr(request),
  );

  protected readonly block = computed(() => this.answer.value()?.block ?? null);
  protected readonly problem = computed(() => this.answer.value()?.problem ?? null);
  protected readonly membership = computed(() => this.answer.value()?.membership ?? null);
  protected readonly splitAnswer = computed(() => this.answer.value()?.split ?? null);
  protected readonly splitChoices = computed(() => this.answer.value()?.splitChoices ?? []);
  protected readonly masks = computed(() => this.answer.value()?.masks ?? []);
  protected readonly planAnswer = computed(() => this.answer.value()?.plan ?? null);
  protected readonly summary = computed(() => this.answer.value()?.summary ?? null);

  protected readonly rows = computed<readonly Row[]>(() => {
    const block = this.block();
    if (!block) return [];
    const values: Record<RowId, string | null> = {
      network: block.family === 'v4' ? block.network : block.cidr,
      mask: block.mask,
      inverseMask: block.inverseMask,
      first: block.first,
      last: block.last,
      broadcast: block.broadcast,
    };
    return ROWS.flatMap((id) => {
      const value = values[id];
      return value === null ? [] : [{ id, value }];
    });
  });

  protected readonly written = computed<readonly Written[]>(() => {
    const block = this.block();
    if (!block) return [];
    const reverse = block.forms.reverse;
    return [
      { id: 'integer', value: block.forms.integer, copyable: true },
      { id: 'expanded', value: block.forms.expanded, copyable: true },
      { id: 'reverse', value: this.reverseText(reverse, this.lang()), copyable: isOneName(reverse) },
    ];
  });

  readonly result = computed<ToolResult | null>(() => {
    const block = this.block();
    if (!block) return null;
    const name = (id: string) => this.transloco.translate(`tools.cidr.rows.${id}.${block.family}`);
    const lines = [
      block.cidr,
      ...this.rows().map((row) => `${name(row.id)} : ${row.value}`),
      `${name('usable')} : ${this.hosts(block)}`,
      ...this.planLines(this.planAnswer()),
      ...this.summaryLines(this.summary()),
    ];
    return {
      title: { key: 'tools.cidr.noteTitle' },
      kind: 'snippet',
      language: 'txt',
      content: lines.join('\n'),
    };
  });

  sample(): void {
    this.family.set(null);
    this.network.set('10.24.8.0/21');
    this.member.set('10.24.12.40');
    this.split.set(null);
    this.plan.set(SAMPLE_PLAN.join('\n'));
    this.list.set(SAMPLE_LIST.join('\n'));
  }

  clear(): void {
    this.network.set('');
    this.member.set('');
    this.split.set(null);
    this.plan.set('');
    this.list.set('');
  }

  /** Digits from Rust, grouped as the reader's language groups them: an IPv6 count passes 2⁵³. */
  protected grouped(digits: string): string {
    return BigInt(digits).toLocaleString(this.lang());
  }

  protected hosts(block: CidrBlock): string {
    return this.transloco.translate(`tools.cidr.usableOf.${block.family}`, {
      usable: this.grouped(block.usable),
      total: this.grouped(block.addresses),
      count: block.addresses === '1' ? 1 : 2,
    });
  }

  /** IPv6 counts read as a power of two: 2¹¹² is a size, its 34 digits are not. */
  protected maskAddresses(row: MaskRow, family: IpFamily): string {
    if (family === 'v4') return this.grouped(row.addresses);
    const exponent = [...String(row.hostBits)].map((digit) => SUPERSCRIPTS[Number(digit)]).join('');
    return `2${exponent}`;
  }

  protected onNetwork(event: Event): void {
    this.network.set((event.target as HTMLInputElement).value);
    this.split.set(null);
  }

  protected onMember(event: Event): void {
    this.member.set((event.target as HTMLInputElement).value);
  }

  protected onPlan(event: Event): void {
    this.plan.set((event.target as HTMLTextAreaElement).value);
  }

  protected onList(event: Event): void {
    this.list.set((event.target as HTMLTextAreaElement).value);
  }

  protected onFamily(id: string): void {
    this.family.set(id === AUTO ? null : (id as IpFamily));
  }

  protected chooseSplit(prefix: number): void {
    this.split.set(prefix);
  }

  protected asLines(blocks: readonly string[]): string {
    return blocks.join('\n');
  }

  /** Puts the prefix on the address as it was typed, so going back finds the same block. */
  protected applyPrefix(prefix: number): void {
    const block = this.block();
    if (!block) return;
    this.network.set(`${block.typed}/${prefix}`);
    this.split.set(null);
  }

  private reverseText(reverse: ReverseDns, lang: string): string {
    switch (reverse.kind) {
      case 'record':
        return reverse.name;
      case 'zones':
        return reverse.count === 1
          ? reverse.first
          : this.transloco.translate(
              'tools.cidr.reverse.zones',
              { count: reverse.count, first: reverse.first, last: reverse.last },
              lang,
            );
      case 'classless':
        return this.transloco.translate('tools.cidr.reverse.classless', { zone: reverse.zone }, lang);
    }
  }

  private planLines(plan: CidrPlan | null): string[] {
    const placed = plan?.lines.flatMap((line) => (line.kind === 'placed' ? [line] : [])) ?? [];
    if (placed.length === 0) return [];
    return [
      '',
      this.transloco.translate('tools.cidr.plan.title'),
      ...placed.map((line) => `${line.name || '—'} : ${line.cidr} (${line.first} – ${line.last})`),
    ];
  }

  private summaryLines(summary: CidrSummary | null): string[] {
    if (!summary || summary.families.length === 0) return [];
    return [
      '',
      this.transloco.translate('tools.cidr.list.title'),
      ...summary.families.flatMap((family) => [
        ...family.blocks,
        `${this.transloco.translate('tools.cidr.list.supernet')} : ${family.supernet}`,
      ]),
    ];
  }
}

function isOneName(reverse: ReverseDns): boolean {
  return reverse.kind === 'record' || (reverse.kind === 'zones' && reverse.count === 1);
}
