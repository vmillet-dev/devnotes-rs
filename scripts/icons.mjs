// Regenerates src-tauri/icons from src-tauri/icons/devnotes.svg: `npm run icons`.
//
// ⚠️ The Windows .ico is not `tauri icon`'s: 16, 24 and 32 px come from the hand-hinted
// drawings in icons/source/, whose lines sit on whole pixels. Scaled from the master they blur,
// and 24 px is the taskbar, 16 px the tray.
import { execFileSync } from 'node:child_process';
import { copyFileSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const ICONS = 'src-tauri/icons';
const HINTED = [16, 24, 32];
const SCALED = [48, 64, 256];

const scratch = mkdtempSync(join(tmpdir(), 'devnotes-icons-'));
const cli = join('node_modules', '@tauri-apps', 'cli', 'tauri.js');
const tauriIcon = (source, output, sizes) =>
  execFileSync(
    process.execPath,
    [cli, 'icon', source, '-o', output, ...(sizes ? ['--png', sizes.join(',')] : [])],
    {
      stdio: 'ignore',
    },
  );

try {
  // Only the files already tracked: `tauri icon` also writes Android, iOS and macOS sets.
  tauriIcon(join(ICONS, 'devnotes.svg'), join(scratch, 'all'));
  const tracked = readdirSync(ICONS).filter((name) => name.endsWith('.png'));
  for (const name of tracked) {
    copyFileSync(join(scratch, 'all', name), join(ICONS, name));
  }

  const png = {};
  for (const size of HINTED) {
    tauriIcon(join(ICONS, 'source', `${size}.svg`), join(scratch, String(size)), [size]);
    png[size] = readFileSync(join(scratch, String(size), `${size}x${size}.png`));
  }
  tauriIcon(join(ICONS, 'devnotes.svg'), join(scratch, 'scaled'), SCALED);
  for (const size of SCALED) {
    png[size] = readFileSync(join(scratch, 'scaled', `${size}x${size}.png`));
  }
  copyFileSync(join(scratch, '32', '32x32.png'), join(ICONS, '32x32.png'));

  writeFileSync(join(ICONS, 'icon.ico'), ico([...HINTED, ...SCALED].map((size) => [size, png[size]])));
} finally {
  rmSync(scratch, { recursive: true, force: true });
}

/** An .ico of PNG entries, which Windows reads since Vista. */
function ico(entries) {
  const header = Buffer.alloc(6 + 16 * entries.length);
  header.writeUInt16LE(0, 0);
  header.writeUInt16LE(1, 2);
  header.writeUInt16LE(entries.length, 4);

  let offset = header.length;
  entries.forEach(([size, data], index) => {
    const at = 6 + 16 * index;
    header.writeUInt8(size >= 256 ? 0 : size, at);
    header.writeUInt8(size >= 256 ? 0 : size, at + 1);
    header.writeUInt16LE(1, at + 4);
    header.writeUInt16LE(32, at + 6);
    header.writeUInt32LE(data.length, at + 8);
    header.writeUInt32LE(offset, at + 12);
    offset += data.length;
  });

  return Buffer.concat([header, ...entries.map(([, data]) => data)]);
}
