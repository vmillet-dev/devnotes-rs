import { smallMode } from './small-mode';

export const prisma = smallMode({
  name: 'prisma',
  keywords: ['model', 'enum', 'datasource', 'generator', 'type', 'view'],
  types: [
    'String',
    'Int',
    'Float',
    'Boolean',
    'DateTime',
    'Json',
    'Bytes',
    'Decimal',
    'BigInt',
    'Unsupported',
  ],
  atoms: ['true', 'false'],
  lineComments: ['//'],
  quotes: ['"'],
  annotation: /^@@?[A-Za-z_][\w.]*/,
});
