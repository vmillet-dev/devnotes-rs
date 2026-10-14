import { Type, provideZonelessChangeDetection } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';

/**
 * The page as a visitor reads it: rendered, settled, and handed back as its DOM. Tests
 * assert what is on screen — headings, landmarks, names, links — never a component's
 * internals, and never an HTML snapshot.
 */
export async function renderPage<T>(
  component: Type<T>,
  inputs: Readonly<Record<string, unknown>> = {},
): Promise<HTMLElement> {
  TestBed.configureTestingModule({
    providers: [provideZonelessChangeDetection(), provideRouter([])],
  });
  const fixture = TestBed.createComponent(component);
  for (const [name, value] of Object.entries(inputs)) fixture.componentRef.setInput(name, value);
  await fixture.whenStable();
  return fixture.nativeElement as HTMLElement;
}

/** The text a screen reader would announce for a control: its label, else its content. */
export function accessibleName(element: Element): string {
  return (element.getAttribute('aria-label') ?? element.textContent ?? '').replace(/\s+/g, ' ').trim();
}
