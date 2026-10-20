import { TestBed } from '@angular/core/testing';
import { Mock, afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { MERMAID_LOADER, MermaidApi, MermaidRenderer, lineOf } from './mermaid-renderer';

describe('MermaidRenderer', () => {
  let api: {
    initialize: Mock<(config: unknown) => void>;
    render: Mock<(id: string, source: string) => Promise<{ svg: string }>>;
  };
  let loads: number;
  let renderer: MermaidRenderer;

  beforeEach(() => {
    loads = 0;
    api = {
      initialize: vi.fn(),
      render: vi.fn(async (id: string, source: string) => {
        if (source.includes('oops'))
          throw Object.assign(new Error('Parse error'), { hash: { loc: { first_line: 2 } } });
        return { svg: `<svg id="${id}">${source}</svg>` };
      }),
    };
    TestBed.configureTestingModule({
      providers: [
        {
          provide: MERMAID_LOADER,
          useValue: () => {
            loads += 1;
            return Promise.resolve(api as unknown as MermaidApi);
          },
        },
      ],
    });
    document.documentElement.dataset['theme'] = 'dark';
    renderer = TestBed.inject(MermaidRenderer);
  });

  afterEach(() => {
    delete document.documentElement.dataset['theme'];
  });

  it('draws a diagram, loading Mermaid once, in the strict level and the app colours', async () => {
    expect(await renderer.render('graph TD; A-->B')).toEqual({
      svg: '<svg id="devnotes-diagram-1">graph TD; A-->B</svg>',
    });
    await renderer.render('graph TD; B-->C');

    expect(loads).toBe(1);
    expect(api.initialize).toHaveBeenCalledTimes(1);
    expect(api.initialize.mock.calls[0][0]).toMatchObject({
      startOnLoad: false,
      securityLevel: 'strict',
      suppressErrorRendering: true,
      theme: 'base',
      themeVariables: { darkMode: true },
    });
  });

  it('answers a diagram already drawn without drawing it again', async () => {
    await renderer.render('graph TD; A-->B');
    await renderer.render('graph TD; A-->B');

    expect(api.render).toHaveBeenCalledTimes(1);
  });

  it('draws again in the other theme, configured for it', async () => {
    await renderer.render('graph TD; A-->B');
    document.documentElement.dataset['theme'] = 'light';
    await renderer.render('graph TD; A-->B');

    expect(api.render).toHaveBeenCalledTimes(2);
    expect(api.initialize.mock.calls[1][0]).toMatchObject({ themeVariables: { darkMode: false } });
  });

  it('draws one diagram at a time', async () => {
    let running = 0;
    let most = 0;
    api.render.mockImplementation(async (id: string) => {
      running += 1;
      most = Math.max(most, running);
      await new Promise((resolve) => setTimeout(resolve));
      running -= 1;
      return { svg: `<svg id="${id}"></svg>` };
    });

    await Promise.all([renderer.render('a'), renderer.render('b'), renderer.render('c')]);

    expect(most).toBe(1);
  });

  it('answers why a diagram cannot be drawn, and on which line', async () => {
    expect(await renderer.render('graph TD\n oops')).toEqual({ error: 'Parse error', line: 2 });
  });

  it('leaves out the line a parse error opens on, said beside it', async () => {
    api.render.mockRejectedValueOnce(
      Object.assign(new Error('Parse error on line 3:\n...A -->\n------^'), {
        hash: { loc: { first_line: 2 } },
      }),
    );

    expect(await renderer.render('graph TD\n  A -->')).toEqual({ error: '...A -->\n------^', line: 2 });
  });

  it('keeps drawing after a failure', async () => {
    api.render.mockRejectedValueOnce('broken');

    expect(await renderer.render('a')).toEqual({ error: 'broken', line: null });
    expect(await renderer.render('b')).toEqual({ svg: '<svg id="devnotes-diagram-2">b</svg>' });
  });
});

describe('lineOf', () => {
  it('reads the location a parser gives, or the line its message names', () => {
    expect(lineOf(Object.assign(new Error('x'), { hash: { loc: { first_line: 4 } } }))).toBe(4);
    expect(lineOf(new Error('Parsing failed: unexpected character at line 3, column 2'))).toBe(3);
    expect(lineOf(new Error('No diagram type detected'))).toBeNull();
    expect(lineOf(null)).toBeNull();
  });
});
