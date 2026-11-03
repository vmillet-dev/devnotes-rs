import { Injectable } from '@angular/core';
import { commands } from '@core/ipc/bindings';
import { unwrap } from '@core/ipc/ipc.error';
import { CalcSheet } from '@core/model/calc.model';

@Injectable({ providedIn: 'root' })
export class CalcRepository {
  /** `today` is the local day, `2026-10-16`: the sheet's `aujourd'hui`. */
  async evaluate(sheet: string, today: string): Promise<CalcSheet> {
    return unwrap('evaluate_calc', await commands.evaluateCalc(sheet, today));
  }
}
