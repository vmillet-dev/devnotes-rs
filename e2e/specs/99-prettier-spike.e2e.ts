import { browser } from '@wdio/globals';
import { readdirSync, writeFileSync } from 'node:fs';

import { canvas } from '../pageobjects/canvas.page.js';

const BLOCK = [
  "import { Injectable, inject } from '@angular/core'",
  "import { HttpClient } from '@angular/common/http'",
  "import type { Invoice, Receipt } from './billing.model'",
  '@Injectable({ providedIn: "root" }) export class InvoicesApi{',
  'private readonly http = inject(HttpClient)',
  'list(customerId: string, page = 1) { return this.http.get<Invoice[]>(`/api/customers/${customerId}/invoices`, { params: { page, limit: 50 } }) }',
  '  pay(invoice: Invoice): Observable<Receipt> {',
  '    return this.http.post<Receipt>(`/api/invoices/${invoice.id}/pay`, {amount: invoice.total, currency: invoice.currency ?? "EUR"})',
  '  }',
  '  async refund<T extends { id: string }>(item: T, reason?: string): Promise<void> {',
  '    const body = { reason: reason ?? "requested", at: new Date().toISOString(), ...item }',
  '    if (!item.id) { throw new Error("no id") } else { await firstValueFrom(this.http.post(`/api/refunds`, body)) }',
  '  }',
  '}',
];

function typescript(lines: number): string {
  const out: string[] = [];
  let i = 0;
  while (out.length < lines) {
    for (const line of BLOCK) out.push(line.replace('InvoicesApi', `InvoicesApi${i}`));
    i++;
  }
  return out.join('\n');
}

const SAMPLES: [string, string][] = [
  ['babel', 'const a = {b:1,c:[1,2,3]}; function f(x){return x*2}'],
  ['json', '{"a":1,"b":[1,2,{"c":null}]}'],
  ['css', 'a{color:red;background:blue} .b > c{margin:0 auto}'],
  ['scss', '$x: 1px; .a{ .b{ margin:$x } }'],
  ['html', '<div><p>hello <b>world</b></p><script>let a=1</script><style>a{color:red}</style></div>'],
  ['markdown', '# Title\n\n* a\n* b\n\n```js\nconst a=1\n```'],
  ['yaml', 'a:   1\nb:\n    - x\n    -   y'],
  ['graphql', 'query { user(id: 1) { name, email } }'],
];

type Answer = {
  ok: boolean;
  loadMs?: number;
  formatMs?: number;
  totalMs?: number;
  message?: string;
  loc?: unknown;
  formatted?: string;
};

async function run(text: string, parser: string): Promise<Answer> {
  return browser.execute(
    (t: string, p: string) =>
      (window as unknown as { prettierSpike: (t: string, p: string) => Promise<Answer> }).prettierSpike(t, p),
    text,
    parser,
  );
}

describe('Prettier spike', () => {
  it('measures', async () => {
    await canvas.open();
    const report: Record<string, unknown> = {};
    const big = typescript(2000);
    report['typescript 2000 first'] = strip(await run(big, 'typescript'));
    report['typescript 2000 second'] = strip(await run(big, 'typescript'));
    report['typescript 2000 third'] = strip(await run(big, 'typescript'));
    report['babel-ts 2000 first'] = strip(await run(big, 'babel-ts'));
    report['babel-ts 2000 second'] = strip(await run(big, 'babel-ts'));
    for (const [parser, text] of SAMPLES) {
      const answer = await run(text, parser);
      report[parser] = { ...strip(answer), formatted: answer.formatted };
    }
    report['syntax error'] = strip(await run('const a = {\n  b: 1,,\n}', 'typescript'));
    report['csp from page load'] = await browser.execute(
      () => (window as unknown as { cspProbe: unknown }).cspProbe,
    );
    report['csp header'] = await browser.execute(async () => {
      const r = await fetch(location.href);
      return r.headers.get('content-security-policy');
    });
    report['csp meta'] = await browser.execute(
      () => document.querySelector('meta[http-equiv]')?.getAttribute('content') ?? null,
    );
    report['csp in worker'] = strip(await run('', 'probe'));
    report['csp violation'] = await browser.execute(
      () => (window as unknown as { cspViolation?: string }).cspViolation ?? null,
    );
    report['csp probe'] = await browser.execute(() => {
      try {
        new Function('return 1')();
        return 'new Function allowed';
      } catch (e) {
        return `new Function refused: ${String(e)}`;
      }
    });
    const workerFile = readdirSync('dist/devnotes/browser').find((f) => f.startsWith('worker-'))!;
    report['worker response csp'] = await browser.execute(async (name: string) => {
      const r = await fetch(new URL(name, location.href));
      return { name, csp: r.headers.get('content-security-policy'), type: r.headers.get('content-type') };
    }, workerFile);
    report['user agent'] = await browser.execute(() => navigator.userAgent);
    writeFileSync('e2e/logs/prettier-spike.json', JSON.stringify(report, null, 2));
    console.log(JSON.stringify(report, (k, v) => (k === 'formatted' ? undefined : v), 2));
    for (const key of [
      'typescript 2000 first',
      'babel-ts 2000 first',
      ...SAMPLES.map(([parser]) => parser),
    ]) {
      if (!(report[key] as Answer).ok) throw new Error(`${key} failed: ${JSON.stringify(report[key])}`);
    }
  });
});

function strip(answer: Answer): Answer {
  const { formatted, ...rest } = answer;
  return { ...rest, ...(formatted ? { lines: formatted.split('\n').length } : {}) } as Answer;
}
