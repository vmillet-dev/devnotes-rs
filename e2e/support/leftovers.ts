import { execFileSync } from 'node:child_process';

import { IDENTIFIER } from './profile.js';

/** The embedded WebDriver server's port: only an e2e build listens on it. */
export const EMBEDDED_PORT = 4445;

/** Waits without an event loop: this runs before WebdriverIO, in a script of its own. */
function pause(ms: number): void {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms);
}

function alive(pid: number): boolean {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

/** Windows: who listens on the port, kept only when it is DevNotes. */
function windowsListeners(): number[] {
  const script =
    `Get-NetTCPConnection -State Listen -LocalPort ${EMBEDDED_PORT} -ErrorAction SilentlyContinue | ` +
    `Select-Object -ExpandProperty OwningProcess -Unique | ` +
    `Where-Object { (Get-Process -Id $_ -ErrorAction SilentlyContinue).ProcessName -eq 'devnotes' }`;
  const out = execFileSync('powershell', ['-NoProfile', '-NonInteractive', '-Command', script], {
    encoding: 'utf8',
  });
  return out.split(/\s+/).filter(Boolean).map(Number);
}

/** Windows: the WebView2 processes still running on the e2e profile, and on no other. */
function windowsWebViews(): number[] {
  const script =
    `Get-CimInstance Win32_Process -Filter "Name='msedgewebview2.exe'" | ` +
    `Where-Object { $_.CommandLine -like '*${IDENTIFIER}*' } | ` +
    `Select-Object -ExpandProperty ProcessId`;
  const out = execFileSync('powershell', ['-NoProfile', '-NonInteractive', '-Command', script], {
    encoding: 'utf8',
  });
  return out.split(/\s+/).filter(Boolean).map(Number);
}

/** Linux: `ss` names the owner of each listening socket the user may see. */
function linuxListeners(): number[] {
  const out = execFileSync('ss', ['-Hltnp', `sport = :${EMBEDDED_PORT}`], { encoding: 'utf8' });
  return [...out.matchAll(/\("devnotes",pid=(\d+)/g)].map((match) => Number(match[1]));
}

/**
 * A run cut short — a terminal closed, the runner killed — can leave its application up,
 * holding the profile and the port, and the next run would fail on both. Found by the port
 * and never by the executable: `tauri dev` runs the same `target/debug/devnotes`.
 *
 * A run that ends, green or red, needs none of this: the service stops what it spawned.
 */
export function stopLeftoverApplication(): void {
  let pids: number[];
  try {
    pids = process.platform === 'win32' ? windowsListeners() : linuxListeners();
  } catch {
    // No way to ask: the wipe that follows says so if anything still holds the profile.
    return;
  }

  for (const pid of pids) {
    console.log(`Stopping the e2e application a previous run left behind (pid ${pid})`);
    signal(pid, 'SIGTERM');
  }

  for (const pid of pids) {
    for (let waited = 0; alive(pid) && waited < 5_000; waited += 100) pause(100);
    if (alive(pid)) signal(pid, 'SIGKILL');
  }

  if (process.platform === 'win32') stopLeftoverWebViews();
}

/**
 * ⚠️ A WebView2 outlives the application that the service stopped by a few seconds, holding
 * its profile under `%LOCALAPPDATA%`: the next scenario on the same runner met `EPERM` wiping it.
 * Found by the profile in its command line, never by the executable: the user's own
 * applications run WebView2 too.
 */
function stopLeftoverWebViews(): void {
  let pids: number[];
  try {
    pids = windowsWebViews();
  } catch {
    return;
  }
  for (const pid of pids) signal(pid, 'SIGKILL');
  for (const pid of pids) {
    for (let waited = 0; alive(pid) && waited < 5_000; waited += 100) pause(100);
  }
}

/** Gone already is as good as stopped. */
function signal(pid: number, name: 'SIGTERM' | 'SIGKILL'): void {
  try {
    if (process.platform === 'win32') {
      // The tree: its WebView2 processes hold the profile too.
      execFileSync('taskkill', ['/PID', String(pid), '/T', '/F'], { stdio: 'ignore' });
    } else {
      process.kill(pid, name);
    }
  } catch {
    // Exited between the look and the signal.
  }
}
