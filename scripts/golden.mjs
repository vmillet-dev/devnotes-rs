// Writes golden/<subject>/ from the `*.golden.spec.ts` files: `npm run golden`.
// One time zone and the write switch, whatever the machine says: two runs give the same bytes.
import { spawnSync } from 'node:child_process';
import { join } from 'node:path';

const cli = join('node_modules', '@angular', 'cli', 'bin', 'ng.js');
const run = spawnSync(process.execPath, [cli, 'test', '--include', '**/*.golden.spec.ts'], {
  stdio: 'inherit',
  env: { ...process.env, GOLDEN_WRITE: '1', TZ: 'Europe/Paris' },
});
process.exit(run.status ?? 1);
