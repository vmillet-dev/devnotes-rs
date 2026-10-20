import { smallMode } from './small-mode';

/** Terraform's configuration language, and HCL's wherever else it is written. */
export const hcl = smallMode({
  name: 'hcl',
  keywords: [
    'resource',
    'data',
    'provider',
    'variable',
    'output',
    'locals',
    'module',
    'terraform',
    'backend',
    'required_providers',
    'dynamic',
    'content',
    'for',
    'for_each',
    'in',
    'if',
    'count',
    'depends_on',
    'lifecycle',
  ],
  types: ['string', 'number', 'bool', 'list', 'map', 'set', 'object', 'tuple', 'any'],
  atoms: ['true', 'false', 'null'],
  lineComments: ['#', '//'],
  blockComment: ['/*', '*/'],
  quotes: ['"'],
});
