import type { Plugin } from 'prettier';
import { format } from 'prettier/standalone';

const PLUGINS: Record<string, () => Promise<Plugin[]>> = {
  babel: () => Promise.all([import('prettier/plugins/babel'), import('prettier/plugins/estree')]),
  'babel-ts': () => Promise.all([import('prettier/plugins/babel'), import('prettier/plugins/estree')]),
  json: () => Promise.all([import('prettier/plugins/babel'), import('prettier/plugins/estree')]),
  typescript: () => Promise.all([import('prettier/plugins/typescript'), import('prettier/plugins/estree')]),
  css: () => Promise.all([import('prettier/plugins/postcss')]),
  scss: () => Promise.all([import('prettier/plugins/postcss')]),
  html: () =>
    Promise.all([
      import('prettier/plugins/html'),
      import('prettier/plugins/postcss'),
      import('prettier/plugins/babel'),
      import('prettier/plugins/estree'),
    ]),
  markdown: () => Promise.all([import('prettier/plugins/markdown')]),
  yaml: () => Promise.all([import('prettier/plugins/yaml')]),
  graphql: () => Promise.all([import('prettier/plugins/graphql')]),
};

addEventListener('message', async (event: MessageEvent<{ id: number; text: string; parser: string }>) => {
  const { id, text, parser } = event.data;
  if (parser === 'probe') {
    let message: string;
    try {
      message = `new Function allowed in worker: ${new Function('return 1')()}`;
    } catch (e) {
      message = `refused in worker: ${String(e)}`;
    }
    postMessage({ id, ok: true, message });
    return;
  }
  const started = performance.now();
  try {
    const plugins = await PLUGINS[parser]!();
    const loaded = performance.now();
    const formatted = await format(text, { parser, plugins });
    postMessage({ id, ok: true, formatted, loadMs: loaded - started, formatMs: performance.now() - loaded });
  } catch (error) {
    postMessage({
      id,
      ok: false,
      message: String(error).slice(0, 200),
      loc: (error as { loc?: unknown }).loc,
    });
  }
});
