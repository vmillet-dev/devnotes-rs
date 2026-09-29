import { describe, expect, it } from 'vitest';
import { DEFAULT_PRETTIER_SETTINGS, FormatRequest, PrettierParser, prettierOptions } from './format.model';
import { loadPlugins } from './prettier-plugins';
import { runPrettier } from './prettier-runner';

/** The real Prettier and its real plugins: the worker is the only thing left out. */
function request(text: string, parser: PrettierParser, indent = '  '): FormatRequest {
  return { text, cursor: 0, parser, options: prettierOptions(DEFAULT_PRETTIER_SETTINGS, indent) };
}

describe('runPrettier', () => {
  it('formats and says which lines it touched', async () => {
    const answer = await runPrettier(request('const a = {b:1}\nconst c = 2;', 'babel'), loadPlugins);

    expect(answer).toEqual({
      kind: 'formatted',
      text: 'const a = { b: 1 };\nconst c = 2;',
      cursor: 0,
      changedLines: [0],
    });
  });

  it('writes the library style: single quotes, semicolons, trailing commas, 100 columns', async () => {
    const long = `call("${'a'.repeat(40)}", "${'b'.repeat(40)}", "${'c'.repeat(20)}")`;
    const answer = await runPrettier(request(long, 'typescript'), loadPlugins);

    expect(answer.kind === 'formatted' && answer.text).toBe(
      `call(\n  '${'a'.repeat(40)}',\n  '${'b'.repeat(40)}',\n  '${'c'.repeat(20)}',\n);`,
    );
  });

  it('indents the way the Tab key does', async () => {
    const answer = await runPrettier(request('if (a) { b() }', 'babel', '\t'), loadPlugins);

    expect(answer.kind === 'formatted' && answer.text).toBe('if (a) {\n\tb();\n}');
  });

  it('keeps a snippet without a final newline without one, and one with it', async () => {
    const bare = await runPrettier(request('a{color:red}', 'css'), loadPlugins);
    const ended = await runPrettier(request('a{color:red}\n', 'css'), loadPlugins);

    expect(bare.kind === 'formatted' && bare.text).toBe('a {\n  color: red;\n}');
    expect(ended.kind === 'formatted' && ended.text).toBe('a {\n  color: red;\n}\n');
  });

  it('keeps CRLF line endings', async () => {
    const answer = await runPrettier(request('a:   1\r\nb:    2', 'yaml'), loadPlugins);

    expect(answer.kind === 'formatted' && answer.text).toBe('a: 1\r\nb: 2');
  });

  it('leaves {{fields}} exactly as they were written', async () => {
    const answer = await runPrettier(
      request('connect({{db_host}},{{ port = 5432 }})\nhost: {{db_host}}', 'babel'),
      loadPlugins,
    );

    expect(answer.kind === 'formatted' && answer.text).toBe(
      'connect({{db_host}}, {{ port = 5432 }});\nhost: {{db_host}};',
    );
  });

  it('keeps a YAML field a value rather than a mapping', async () => {
    const answer = await runPrettier(request('host:   {{db_host}}', 'yaml'), loadPlugins);

    expect(answer.kind === 'formatted' && answer.text).toBe('host: {{db_host}}');
  });

  it('answers where a syntax error is, and changes nothing', async () => {
    const answer = await runPrettier(request('const a = {\n  b: 1,,\n}', 'typescript'), loadPlugins);

    expect(answer).toEqual({ kind: 'syntax', line: 2, column: 8 });
  });

  it('says so when there was nothing to do', async () => {
    expect(await runPrettier(request('const a = 1;', 'babel'), loadPlugins)).toEqual({ kind: 'unchanged' });
  });

  it('formats every language it offers', async () => {
    const samples: [PrettierParser, string][] = [
      ['json', '{"a":1}'],
      ['scss', '$x: 1px; .a{ .b{ margin:$x } }'],
      ['html', '<div><p>hi</p><script>let a=1</script></div>'],
      ['markdown', '* a\n* b'],
      ['graphql', 'query { user(id: 1) { name, email } }'],
    ];

    for (const [parser, text] of samples) {
      expect((await runPrettier(request(text, parser), loadPlugins)).kind, parser).toBe('formatted');
    }
  });

  it('fails without a syntax location on anything else', async () => {
    const broken = () => Promise.reject(new Error('no plugin'));

    expect(await runPrettier(request('a', 'babel'), broken)).toEqual({ kind: 'failed' });
  });

  it('refuses a text whose every marker is taken', async () => {
    const letters = 'qzjkvw';
    const every = [...letters].flatMap((a) => [...letters].map((b) => a + b)).join(' ');

    expect(await runPrettier(request(every, 'babel'), loadPlugins)).toEqual({ kind: 'failed' });
  });
});
