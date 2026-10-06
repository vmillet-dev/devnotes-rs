import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { ToolsRepository } from '@core/data/tools.repository';
import { CidrBlock, IpFamily } from '@core/model/tool-answers.model';
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

/** The card's rows, in its order; IPv6 has neither mask nor broadcast. */
const ROWS = ['network', 'mask', 'inverseMask', 'first', 'last', 'broadcast'] as const;
type RowId = (typeof ROWS)[number];

interface Row {
  readonly id: RowId;
  readonly value: string;
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

  protected readonly families: readonly Segment[] = [AUTO, 'v4', 'v6'].map((id) => ({
    id,
    labelKey: `tools.cidr.families.${id}`,
  }));

  protected readonly answer = liveResult(
    () => ({ network: this.network(), family: this.family(), member: this.member(), split: this.split() }),
    (request) => this.repository.describeCidr(request),
  );

  protected readonly block = computed(() => this.answer.value()?.block ?? null);
  protected readonly problem = computed(() => this.answer.value()?.problem ?? null);
  protected readonly membership = computed(() => this.answer.value()?.membership ?? null);
  protected readonly splitAnswer = computed(() => this.answer.value()?.split ?? null);
  protected readonly splitChoices = computed(() => this.answer.value()?.splitChoices ?? []);

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

  readonly result = computed<ToolResult | null>(() => {
    const block = this.block();
    if (!block) return null;
    const name = (id: string) => this.transloco.translate(`tools.cidr.rows.${id}.${block.family}`);
    const lines = [
      block.cidr,
      ...this.rows().map((row) => `${name(row.id)} : ${row.value}`),
      `${name('usable')} : ${this.hosts(block)}`,
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
  }

  clear(): void {
    this.network.set('');
    this.member.set('');
    this.split.set(null);
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

  protected onNetwork(event: Event): void {
    this.network.set((event.target as HTMLInputElement).value);
    this.split.set(null);
  }

  protected onMember(event: Event): void {
    this.member.set((event.target as HTMLInputElement).value);
  }

  protected onFamily(id: string): void {
    this.family.set(id === AUTO ? null : (id as IpFamily));
  }

  protected chooseSplit(prefix: number): void {
    this.split.set(prefix);
  }
}
