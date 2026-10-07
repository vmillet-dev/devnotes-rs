import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { UrlFieldComponent } from './url-field.component';

describe('UrlFieldComponent', () => {
  let fixture: ComponentFixture<UrlFieldComponent>;
  let emitted: string[];

  beforeEach(async () => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({ imports: [UrlFieldComponent] });
    fixture = TestBed.createComponent(UrlFieldComponent);
    fixture.componentRef.setInput('value', '{{baseUrl}}/customers/{{customerId}}?page=1');
    fixture.componentRef.setInput('label', 'URL');
    emitted = [];
    fixture.componentInstance.valueChange.subscribe((value) => emitted.push(value));
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  const el = <E extends HTMLElement = HTMLElement>(selector: string): E =>
    fixture.nativeElement.querySelector(selector);

  it('lights each variable in the copy drawn under the field, the rest as typed', () => {
    const mirror = el('[data-testid="http-url-mirror"]');

    expect(
      [...mirror.querySelectorAll('[data-testid="http-url-variable"]')].map((piece) => piece.textContent),
    ).toEqual(['{{baseUrl}}', '{{customerId}}']);
    expect(mirror.textContent).toBe('{{baseUrl}}/customers/{{customerId}}?page=1');
  });

  it('hands on what is typed, and keeps the copy scrolled with the field', () => {
    const field = el<HTMLInputElement>('[data-testid="http-url"]');
    field.value = '{{baseUrl}}/x';
    Object.defineProperty(field, 'scrollLeft', { value: 40, configurable: true });
    field.dispatchEvent(new Event('input'));

    expect(emitted).toEqual(['{{baseUrl}}/x']);
    expect(el('[data-testid="http-url-mirror"]').scrollLeft).toBe(40);
  });
});
