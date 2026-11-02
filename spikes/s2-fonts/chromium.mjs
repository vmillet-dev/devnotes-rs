// Renders samples.tsv in Chromium (Blink, the engine of WebView2) with the same TTF files,
// one PNG per sample and scale: out/s2/chromium/<scale>/<id>.png. Run from spikes/s2-fonts.
import { readFileSync, mkdirSync } from 'node:fs';
import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
const { chromium } = require('playwright');

const samples = readFileSync('samples.tsv', 'utf8').trim().split('\n').map((l) => {
  const [id, family, size, weight, spacing, width, rows, text] = l.split('\t');
  return { id, family, size: +size, weight: +weight, spacing: +spacing, width: +width, rows: +rows, text };
});
// Inline: a page set by setContent is about:blank, which may not load file:// URLs.
const data = (file) => 'data:font/ttf;base64,' + readFileSync(new URL('fonts/' + file, import.meta.url)).toString('base64');
const css = `
@font-face { font-family: InterSpike; src: url(${data('InterVariable-latin.ttf')}); font-weight: 100 900; }
@font-face { font-family: InterSpike; src: url(${data('InterVariable-latin-ext.ttf')}); font-weight: 100 900; unicode-range: U+0100-024F, U+1E00-1EFF; }
@font-face { font-family: JBMonoSpike; src: url(${data('JetBrainsMonoVariable-latin.ttf')}); font-weight: 100 800; }
body { margin: 0; background: rgb(22,22,24); }
.s { box-sizing: border-box; padding: 4px; background: rgb(22,22,24); color: rgb(232,232,232); overflow: hidden; }
.t { overflow: hidden; }
.one { white-space: nowrap; text-overflow: ellipsis; }
.many { display: -webkit-box; -webkit-box-orient: vertical; overflow-wrap: anywhere; }`;
const browser = await chromium.launch();
for (const scale of [1, 1.25, 1.5]) {
  const page = await browser.newPage({ deviceScaleFactor: scale, viewport: { width: 800, height: 200 } });
  mkdirSync(`../out/s2/chromium/${scale * 100}`, { recursive: true });
  for (const s of samples) {
    const height = 8 + s.size * 1.5 * s.rows;
    const family = s.family === 'mono' ? 'JBMonoSpike' : 'InterSpike';
    await page.setContent(`<style>${css}</style><div class="s" style="width:${s.width + 8}px;height:${height}px">
      <div class="t ${s.rows === 1 ? 'one' : 'many'}" style="font-family:${family}, sans-serif;font-size:${s.size}px;font-weight:${s.weight};
      letter-spacing:${s.spacing}px;line-height:${s.size * 1.5}px;width:${s.width}px;-webkit-line-clamp:${s.rows}">${s.text.replace(/</g, '&lt;')}</div></div>`);
    const loaded = await page.evaluate(async () => { await document.fonts.ready; return [...document.fonts].filter((f) => f.status === 'loaded').length; });
    if (loaded === 0) throw new Error('no font loaded for ' + s.id);
    await page.locator('.s').screenshot({ path: `../out/s2/chromium/${scale * 100}/${s.id}.png` });
  }
}
await browser.close();
