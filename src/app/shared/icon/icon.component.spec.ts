import { TestBed } from '@angular/core/testing';
import { describe, expect, it } from 'vitest';
import { IconComponent, IconName } from './icon.component';

const SVG = 'http://www.w3.org/2000/svg';

describe('IconComponent', () => {
  function render(name: IconName): HTMLElement {
    const fixture = TestBed.createComponent(IconComponent);
    fixture.componentRef.setInput('name', name);
    fixture.detectChanges();
    return fixture.nativeElement as HTMLElement;
  }

  /** ⚠️ An HTML \`<path>\` inside an \`@for\` draws nothing and fails no other assertion. */
  it('draws its paths in the SVG namespace', () => {
    const paths = render('trash').querySelectorAll('path');

    expect(paths.length).toBeGreaterThan(0);
    for (const path of paths) {
      expect(path.namespaceURI).toBe(SVG);
      expect(path.getAttribute('d')).toMatch(/^M/);
    }
  });

  it('strokes in the colour of the text around it', () => {
    expect(render('filter').querySelector('svg')?.getAttribute('stroke')).toBe('currentColor');
  });

  it('stays out of the accessibility tree', () => {
    expect(render('sidebar').querySelector('svg')?.getAttribute('aria-hidden')).toBe('true');
  });
});
