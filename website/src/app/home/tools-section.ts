import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  ElementRef,
  afterNextRender,
  inject,
  signal,
  viewChildren,
} from '@angular/core';

import { canObserve, prefersReducedMotion, sleep } from '../core/browser';

interface Tool {
  readonly name: string;
  /** Its category, as the application's catalogue names it. */
  readonly category: string;
  readonly input: string;
  /** What the tile shows before it runs, and without a script: the same answer, computed once. */
  readonly output: string;
  readonly run?: (input: string) => string | Promise<string>;
}

const encode = (text: string) => btoa(String.fromCharCode(...new TextEncoder().encode(text)));
const base64url = (value: object) =>
  encode(JSON.stringify(value)).replace(/=+$/, '').replace(/\+/g, '-').replace(/\//g, '_');
const JWT = `${base64url({ alg: 'HS256', typ: 'JWT' })}.${base64url({ sub: '42', name: 'Ada', exp: 1767225600 })}.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c`;
const day = new Intl.DateTimeFormat('fr-FR', { dateStyle: 'long', timeZone: 'UTC' });

async function sha256(text: string): Promise<string> {
  const digest = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(text));
  return [...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, '0')).join('');
}

function readJwt(token: string): string {
  const payload = JSON.parse(atob((token.split('.')[1] ?? '').replace(/-/g, '+').replace(/_/g, '/'))) as {
    exp: number;
  };
  const expiry = new Date(payload.exp * 1000);
  const state =
    expiry < new Date() ? `Expiré depuis le ${day.format(expiry)}` : `Expire le ${day.format(expiry)}`;
  const shown = JSON.stringify(payload)
    .replace(/,/g, ', ')
    .replace(/:/g, ': ')
    .replace('{', '{ ')
    .replace(/}$/, ' }');
  return `${shown}\n${state}`;
}

function colour(hex: string): string {
  const [r, g, b] = [1, 3, 5].map((at) => parseInt(hex.slice(at, at + 2), 16)) as [number, number, number];
  const [rr, gg, bb] = [r / 255, g / 255, b / 255];
  const max = Math.max(rr, gg, bb);
  const min = Math.min(rr, gg, bb);
  const lightness = (max + min) / 2;
  const range = max - min;
  const saturation = range === 0 ? 0 : range / (1 - Math.abs(2 * lightness - 1));
  let hue = 0;
  if (range !== 0) {
    if (max === rr) hue = 60 * (((gg - bb) / range) % 6);
    else if (max === gg) hue = 60 * ((bb - rr) / range + 2);
    else hue = 60 * ((rr - gg) / range + 4);
  }
  return `rgb(${r}, ${g}, ${b})\nhsl(${Math.round((hue + 360) % 360)}, ${Math.round(saturation * 100)}%, ${Math.round(lightness * 100)}%)`;
}

function subnet(cidr: string): string {
  const [ip = '', bits = '32'] = cidr.split('/');
  const address = ip.split('.').reduce((value, octet) => value * 256 + Number(octet), 0);
  const size = 2 ** (32 - Number(bits));
  const base = address - (address % size);
  const dotted = (value: number) =>
    [24, 16, 8, 0].map((shift) => Math.floor(value / 2 ** shift) % 256).join('.');
  return `Masque ${dotted(2 ** 32 - size)}\n${(size - 2).toLocaleString('fr-FR')} hôtes, de ${dotted(base + 1)} à ${dotted(base + size - 2)}`;
}

const TOOLS: readonly Tool[] = [
  { name: 'Base64', category: 'Encodeurs', input: 'devnotes', output: 'ZGV2bm90ZXM=', run: encode },
  {
    name: 'Hash',
    category: 'Crypto',
    input: 'devnotes',
    output: '291cb8078ee3f3b74019731cf53f8c41214e6d89cffcf7ba7cca242f5deb1f7c',
    run: sha256,
  },
  {
    name: 'Cron',
    category: 'Temps',
    input: '*/15 9-18 * * 1-5',
    output: 'Toutes les 15 minutes, de 9 h à 18 h 45, du lundi au vendredi',
  },
  {
    name: 'Couleurs',
    category: 'Encodeurs',
    input: '#e8a33d',
    output: 'rgb(232, 163, 61)\nhsl(36, 79%, 57%)',
    run: colour,
  },
  {
    name: 'JWT',
    category: 'Crypto',
    input: `${JWT.slice(0, 64)}…`,
    output: '{ "sub": "42", "name": "Ada", "exp": 1767225600 }\nExpiré depuis le 1 janvier 2026',
    run: () => readJwt(JWT),
  },
  {
    name: 'Identifiants',
    category: 'Crypto',
    input: 'UUID v4',
    output: '3f2b8c1e-7a4d-4e9b-9c61-2d5f0a8e4b17',
    run: () => crypto.randomUUID(),
  },
  {
    name: 'Sous-réseaux CIDR',
    category: 'Calcul',
    input: '10.0.0.0/22',
    output: 'Masque 255.255.252.0\n1 022 hôtes, de 10.0.0.1 à 10.0.3.254',
    run: subnet,
  },
  {
    name: 'URL',
    category: 'Encodeurs',
    input: 'a b&c=d/é',
    output: 'a%20b%26c%3Dd%2F%C3%A9',
    run: encodeURIComponent,
  },
];

/** A tile's state while it plays: the input typed in, then its answer. */
interface Shown {
  readonly input: string;
  readonly output: string;
  readonly typing: boolean;
  readonly waiting: boolean;
}

/** A row of tools that compute for real once in view: the browser stands in for the application. */
@Component({
  selector: 'app-tools-section',
  templateUrl: './tools-section.html',
  styleUrl: './tools-section.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ToolsSection {
  protected readonly tools = TOOLS;
  protected readonly shown = signal<readonly Shown[]>(
    TOOLS.map((tool) => ({ input: tool.input, output: tool.output, typing: false, waiting: false })),
  );
  private readonly tiles = viewChildren<ElementRef<HTMLElement>>('tile');

  constructor() {
    const destroyRef = inject(DestroyRef);
    afterNextRender(() => {
      if (!canObserve()) return;
      const watch = new IntersectionObserver(
        (entries) => {
          for (const entry of entries) {
            if (!entry.isIntersecting) continue;
            watch.unobserve(entry.target);
            const index = this.tiles().findIndex((tile) => tile.nativeElement === entry.target);
            if (index >= 0) void this.play(index);
          }
        },
        { threshold: 0.6 },
      );
      this.tiles().forEach((tile) => watch.observe(tile.nativeElement));
      destroyRef.onDestroy(() => watch.disconnect());
    });
  }

  private async play(index: number): Promise<void> {
    const tool = TOOLS[index];
    if (!tool) return;
    const show = (state: Shown) =>
      this.shown.update((all) => all.map((each, i) => (i === index ? state : each)));

    let output = tool.output;
    try {
      if (tool.run) output = await tool.run(tool.input);
    } catch {
      // A browser without `crypto.subtle` (an insecure origin): the precomputed answer stands.
    }
    if (prefersReducedMotion()) {
      show({ input: tool.input, output, typing: false, waiting: false });
      return;
    }

    // One tile after another along the row, not all at once.
    await sleep((index % 4) * 160);
    show({ input: '', output: '', typing: true, waiting: true });
    const stride = Math.max(1, Math.ceil(tool.input.length / 24));
    for (let n = 1; n <= tool.input.length; n += stride) {
      show({ input: tool.input.slice(0, n), output: '', typing: true, waiting: true });
      await sleep(28);
    }
    show({ input: tool.input, output: '', typing: false, waiting: true });
    await sleep(180);
    show({ input: tool.input, output, typing: false, waiting: false });
  }
}
