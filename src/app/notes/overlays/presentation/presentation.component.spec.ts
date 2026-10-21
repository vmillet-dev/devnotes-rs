import { ComponentFixture, TestBed } from '@angular/core/testing';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { Presentation } from '@core/model/note.model';
import { DiagramResult, MermaidRenderer } from '@core/services/diagrams/mermaid-renderer';
import { FakeAppWindow } from '@testing/fake-app-window';
import { provideAppTesting } from '@testing/testing.providers';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { PresentationComponent } from './presentation.component';

const FENCE = '```';

const SLIDES: Presentation = {
  slides: ['Why we ship.', '## Build\n\n### Rust', '## Ship'],
  map: [
    { text: 'Build', slide: 1, children: [{ text: 'Rust', slide: 1, children: [] }] },
    { text: 'Ship', slide: 2, children: [] },
  ],
};

describe('PresentationComponent', () => {
  let fixture: ComponentFixture<PresentationComponent>;
  let screen: FakeAppWindow;
  let closed: number;
  let drawing: DiagramResult;

  const el = <T extends HTMLElement = HTMLElement>(selector: string): T | null =>
    (fixture.nativeElement as HTMLElement).querySelector<T>(selector);
  const count = () => el('[data-testid="presentation-count"]')?.textContent?.trim();
  const slide = () => el('[data-testid="presentation-slide"]');

  async function press(key: string, init: KeyboardEventInit = {}): Promise<KeyboardEvent> {
    const event = new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...init });
    document.dispatchEvent(event);
    await fixture.whenStable();
    return event;
  }

  async function mount(presentation: Presentation = SLIDES): Promise<void> {
    fixture = TestBed.createComponent(PresentationComponent);
    fixture.componentRef.setInput('title', 'Release');
    fixture.componentRef.setInput('presentation', presentation);
    fixture.componentInstance.closed.subscribe(() => (closed += 1));
    document.body.appendChild(fixture.nativeElement);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  }

  beforeEach(async () => {
    screen = new FakeAppWindow();
    closed = 0;
    drawing = { svg: '<svg><text>drawn</text></svg>' };
    TestBed.configureTestingModule({
      imports: [PresentationComponent],
      providers: [
        provideAppTesting({ appWindow: screen }),
        provideTranslocoTesting(),
        { provide: MermaidRenderer, useValue: { render: () => Promise.resolve(drawing) } },
      ],
    });
  });

  afterEach(() => {
    fixture.destroy();
    fixture.nativeElement.remove();
  });

  it('opens on the title, full screen, then a key a slide', async () => {
    await mount();

    expect(el('[data-testid="presentation-cover"]')?.textContent).toBe('Release');
    expect(count()).toBe('1 / 4');
    await vi.waitFor(() => expect(screen.fullscreen).toBe(true));

    const next = await press('ArrowRight');
    expect(next.defaultPrevented).toBe(true);
    expect(count()).toBe('2 / 4');
    expect(el('[data-testid="presentation-cover"]')).toBeNull();
    expect(slide()?.textContent).toContain('Why we ship.');

    await press('PageDown');
    expect(slide()?.querySelector('h2')?.textContent).toBe('Build');
    expect(slide()?.querySelector('h3')?.textContent).toBe('Rust');
  });

  it('goes back, to either end, and no further', async () => {
    await mount();

    await press('End');
    expect(count()).toBe('4 / 4');
    await press(' ');
    expect(count()).toBe('4 / 4');
    await press('ArrowLeft');
    expect(count()).toBe('3 / 4');
    await press('Home');
    expect(count()).toBe('1 / 4');
    await press('Backspace');
    expect(count()).toBe('1 / 4');
  });

  it('moves from its own buttons too, which stop at either end', async () => {
    await mount();
    const previous = el<HTMLButtonElement>('[data-testid="presentation-previous"]')!;
    const next = el<HTMLButtonElement>('[data-testid="presentation-next"]')!;
    expect(previous.disabled).toBe(true);

    next.click();
    await fixture.whenStable();
    expect(count()).toBe('2 / 4');
    previous.click();
    await fixture.whenStable();
    expect(count()).toBe('1 / 4');
  });

  it('leaves alone a key with a modifier, and Enter on a button it is for', async () => {
    await mount();

    const held = await press('ArrowRight', { ctrlKey: true });
    expect(held.defaultPrevented).toBe(false);
    el<HTMLButtonElement>('[data-testid="presentation-map"]')!.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true }),
    );
    await fixture.whenStable();
    await press('q');

    expect(count()).toBe('1 / 4');
  });

  it('turns to the map of its headings with M, and opens the slide of the one pressed', async () => {
    await mount();

    await press('m');
    const nodes = [...fixture.nativeElement.querySelectorAll('[data-testid="presentation-map-node"]')];
    expect(nodes.map((node) => (node as HTMLElement).textContent?.trim())).toEqual([
      'Release',
      'Build',
      'Rust',
      'Ship',
    ]);
    await press('ArrowRight');
    expect(count()).toBe('1 / 4');

    (nodes[3] as HTMLButtonElement).click();
    await fixture.whenStable();
    expect(el('[data-testid="presentation-map-node"]')).toBeNull();
    expect(count()).toBe('4 / 4');

    await press('M');
    (
      fixture.nativeElement.querySelector('[data-testid="presentation-map-node"]') as HTMLButtonElement
    ).click();
    await fixture.whenStable();
    expect(count()).toBe('1 / 4');
  });

  it('leaves on Escape, and gives the screen back', async () => {
    await mount();
    await vi.waitFor(() => expect(screen.fullscreen).toBe(true));

    await press('Escape');
    expect(closed).toBe(1);

    fixture.destroy();
    await vi.waitFor(() => expect(screen.fullscreen).toBe(false));
  });

  it('keeps a window full screen that already was', async () => {
    screen.fullscreen = true;
    await mount();

    el<HTMLButtonElement>('[data-testid="presentation-close"]')!.click();
    expect(closed).toBe(1);
    fixture.destroy();
    await new Promise((resolve) => setTimeout(resolve));

    expect(screen.fullscreen).toBe(true);
  });

  it('draws a slide’s diagram alone, and says when it cannot be read', async () => {
    drawing = { error: 'Parse error', line: 2 };
    await mount({ slides: [`${FENCE}mermaid\ngraph TD\n  A -->\n${FENCE}`], map: [] });

    await press('ArrowRight');

    await vi.waitFor(() =>
      expect(slide()?.querySelector('.rich-diagram-error')?.textContent).toBe(
        'Le diagramme ne se lit pas, ligne 2.',
      ),
    );
    expect(slide()?.querySelector('pre')?.classList).toContain('is-folded');
  });

  it('says so when the line cannot be told', async () => {
    drawing = { error: 'No diagram type detected', line: null };
    await mount({ slides: [`${FENCE}mermaid\nnot a diagram\n${FENCE}`], map: [] });

    await press('ArrowRight');

    await vi.waitFor(() =>
      expect(slide()?.querySelector('.rich-diagram-error')?.textContent).toBe('Le diagramme ne se lit pas.'),
    );
  });
});
