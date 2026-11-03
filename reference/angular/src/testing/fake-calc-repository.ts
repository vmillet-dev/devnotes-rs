import { CalcRepository } from '@core/data/calc.repository';
import { CalcSheet } from '@core/model/calc.model';
import { guard } from './fail-next';

/** What Rust answers is set by the spec: the double computes nothing. */
export class FakeCalcRepository implements Pick<CalcRepository, keyof CalcRepository> {
  sheet: CalcSheet = { rows: [], variables: [] };
  readonly asked: { sheet: string; today: string }[] = [];
  failNext: Error | null = null;

  evaluate(sheet: string, today: string): Promise<CalcSheet> {
    this.asked.push({ sheet, today });
    return guard(this, () => this.sheet);
  }
}
