import { ChangeDetectionStrategy, Component, computed, input, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { TestResult } from '@core/model/http.model';
import { TranslationRef } from '@core/services/i18n/translation-ref.model';

/** The tests of one script holder, in the order they ran. */
interface TestGroup {
  readonly key: string;
  readonly label: TranslationRef;
  readonly tests: readonly TestResult[];
}

/** The response's Tests tab: how many passed, then each test under whose it is. */
@Component({
  selector: 'app-response-tests',
  imports: [TranslocoPipe],
  templateUrl: './response-tests.component.html',
  styleUrl: './response-tests.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ResponseTestsComponent {
  readonly tests = input.required<readonly TestResult[]>();
  readonly requestName = input.required<string>();
  /** « Ligne 8 › »: not from the history, whose request may have changed since. */
  readonly openable = input(false);

  readonly lineOpened = output<TestResult>();

  protected readonly passed = computed(() => this.tests().filter((test) => test.passed).length);
  protected readonly failed = computed(
    () => this.tests().filter((test) => !test.passed && !test.skipped).length,
  );
  protected readonly ran = computed(() => this.passed() + this.failed());

  protected readonly groups = computed<readonly TestGroup[]>(() => {
    const groups = new Map<string, { label: TranslationRef; tests: TestResult[] }>();
    for (const test of this.tests()) {
      const level = test.level ?? 'request';
      const key = `${level}:${test.owner ?? ''}`;
      const group = groups.get(key) ?? { label: this.labelOf(test), tests: [] };
      group.tests.push(test);
      groups.set(key, group);
    }
    return [...groups].map(([key, group]) => ({ key, ...group }));
  });

  private labelOf(test: TestResult): TranslationRef {
    const level = test.level ?? 'request';
    return level === 'request'
      ? { key: 'http.tests.groups.request', params: { name: this.requestName() } }
      : { key: `http.tests.groups.${level}`, params: { name: test.holder ?? '' } };
  }
}
