import type { Plugin } from 'prettier';
import { PrettierParser } from './format.model';

const babel = () => Promise.all([import('prettier/plugins/babel'), import('prettier/plugins/estree')]);

/** One chunk per plugin, loaded by the first format that needs it and never at launch. */
export async function loadPlugins(parser: PrettierParser): Promise<Plugin[]> {
  switch (parser) {
    case 'babel':
    case 'json':
      return babel();
    case 'typescript':
      return Promise.all([import('prettier/plugins/typescript'), import('prettier/plugins/estree')]);
    case 'css':
    case 'scss':
    case 'less':
      return [await import('prettier/plugins/postcss')];
    // Its `<script>` and `<style>` are formatted too, by the plugins of their languages.
    case 'html':
      return [
        ...(await babel()),
        await import('prettier/plugins/html'),
        await import('prettier/plugins/postcss'),
      ];
    case 'vue':
      return [
        ...(await babel()),
        await import('prettier/plugins/typescript'),
        await import('prettier/plugins/html'),
        await import('prettier/plugins/postcss'),
      ];
    // Its bindings and control flow are expressions of Angular's own, which its plugin reads.
    case 'angular':
      return [
        await import('prettier/plugins/html'),
        await import('prettier/plugins/angular'),
        await import('prettier/plugins/postcss'),
      ];
    case 'markdown':
      return [await import('prettier/plugins/markdown')];
    case 'yaml':
      return [await import('prettier/plugins/yaml')];
    case 'graphql':
      return [await import('prettier/plugins/graphql')];
  }
}
