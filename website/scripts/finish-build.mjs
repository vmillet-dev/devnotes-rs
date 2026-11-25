// After `ng build website`: the files a static host reads beside the pages (the sitemap,
// robots.txt, a 404 page at the root), then a check that every page was written as real
// HTML. The site's address is read back from the home page's canonical link, so it is set
// in one place: the build's `SITE_URL`.
import { copyFileSync, existsSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const OUT = 'dist';

const routes = JSON.parse(readFileSync(join(OUT, 'prerendered-routes.json'), 'utf8')).routes;
const paths = Object.keys(routes).filter((path) => path !== '/404');

const page = (path) => join(OUT, path === '/' ? '' : path.slice(1), 'index.html');
const home = readFileSync(page('/'), 'utf8');
const origin = /<link rel="canonical" href="([^"]+)"/.exec(home)?.[1]?.replace(/\/$/, '');
if (!origin) throw new Error('The home page has no canonical link to read the site address from.');

const problems = [];
for (const path of paths) {
  const html = readFileSync(page(path), 'utf8');
  const h1 = html.match(/<h1[\s>]/g)?.length ?? 0;
  if (h1 !== 1) problems.push(`${path}: ${h1} <h1>`);
  if (!html.includes('<link rel="canonical"')) problems.push(`${path}: no canonical link`);
  if (/https?:\/\/(localhost|127\.0\.0\.1)/.test(html)) problems.push(`${path}: a local address`);
  if (!/<title>[^<]+<\/title>/.test(html)) problems.push(`${path}: no title`);
}
if (!existsSync(join(OUT, 'index.html'))) problems.push('no index.html at the root');

// Each page names both its languages, itself included, and they name each other.
const alternates = (path) =>
  Object.fromEntries(
    [
      ...readFileSync(page(path), 'utf8').matchAll(
        /<link rel="alternate" hreflang="([^"]+)" href="([^"]+)"/g,
      ),
    ].map(([, hreflang, href]) => [hreflang, href.slice(origin.length) || '/']),
  );
for (const path of paths) {
  const { fr, en, 'x-default': fallback } = alternates(path);
  if (!paths.includes(fr) || !paths.includes(en)) problems.push(`${path}: its languages are ${fr} and ${en}`);
  else if (![fr, en].includes(path) || fallback !== fr) problems.push(`${path}: is neither of its languages`);
  else if (alternates(fr).en !== en || alternates(en).fr !== fr)
    problems.push(`${path}: its languages disagree`);
}

// What a search engine and a link preview read: structured data that parses, an image that exists.
for (const path of paths) {
  const html = readFileSync(page(path), 'utf8');
  for (const [, data] of html.matchAll(/<script type="application\/ld\+json">([\s\S]*?)<\/script>/g)) {
    try {
      JSON.parse(data);
    } catch (error) {
      problems.push(`${path}: structured data that does not parse (${error.message})`);
    }
  }
  const image = /<meta property="og:image" content="([^"]+)"/.exec(html)?.[1];
  const file = image && join(OUT, image.slice(origin.length));
  if (!file || !existsSync(file)) problems.push(`${path}: no social image (${image})`);
  else {
    const png = readFileSync(file);
    const size = `${png.readUInt32BE(16)}×${png.readUInt32BE(20)}`;
    if (size !== '1200×630') problems.push(`${path}: a social image of ${size}`);
  }
}

// The download links name the version's own files: built before that release is published,
// the site would offer files that do not exist yet.
const files = new Set(
  paths.flatMap((path) =>
    [
      ...readFileSync(page(path), 'utf8').matchAll(
        /href="(https:\/\/github\.com\/[^"]+\/releases\/download\/[^"]+)"/g,
      ),
    ].map((match) => match[1]),
  ),
);
for (const file of files) {
  const answer = await fetch(file, { method: 'HEAD', redirect: 'follow' }).catch((error) => ({
    status: error.message,
  }));
  if (answer.status !== 200) problems.push(`${file}: ${answer.status}`);
}
if (!files.size) problems.push('no download link to a release file');

if (problems.length) {
  console.error(problems.join('\n'));
  process.exit(1);
}

// No date of the build in the sitemap: the site is published when its files change.
const url = (path) => `${origin}${path === '/' ? '/' : path}`;
const languages = (path) =>
  Object.entries(alternates(path))
    .map(([hreflang, href]) => `<xhtml:link rel="alternate" hreflang="${hreflang}" href="${url(href)}"/>`)
    .join('');
writeFileSync(
  join(OUT, 'sitemap.xml'),
  `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">\n${paths
    .map((path) => `  <url><loc>${url(path)}</loc>${languages(path)}</url>`)
    .join('\n')}\n</urlset>\n`,
);
writeFileSync(join(OUT, 'robots.txt'), `User-agent: *\nAllow: /\n\nSitemap: ${origin}/sitemap.xml\n`);
copyFileSync(page('/404'), join(OUT, '404.html'));

// Build bookkeeping a static host has no use for: the client-rendered shell and the route list.
rmSync(join(OUT, 'index.csr.html'), { force: true });
rmSync(join(OUT, 'prerendered-routes.json'), { force: true });

console.log(`${paths.length} pages checked for ${origin}; sitemap.xml, robots.txt and 404.html written.`);
