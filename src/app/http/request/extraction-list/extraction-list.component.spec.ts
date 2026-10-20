import { ComponentFixture, TestBed } from '@angular/core/testing';
import { describe, expect, it } from 'vitest';
import { provideAppTesting } from '@testing/testing.providers';
import { ExtractionListComponent } from './extraction-list.component';

describe('ExtractionListComponent', () => {
  it('lists each path into its variable, marks a secret, and removes one', async () => {
    TestBed.configureTestingModule({ imports: [ExtractionListComponent], providers: [provideAppTesting()] });
    const fixture: ComponentFixture<ExtractionListComponent> =
      TestBed.createComponent(ExtractionListComponent);
    fixture.componentRef.setInput('rules', [
      { path: 'body.access_token', variable: 'accessToken', secret: true },
      { path: 'body.expires_in', variable: 'tokenTtl', secret: false },
    ]);
    const removed: number[] = [];
    fixture.componentInstance.removed.subscribe((index) => removed.push(index));
    fixture.autoDetectChanges();
    await fixture.whenStable();

    const rows = [
      ...(fixture.nativeElement as HTMLElement).querySelectorAll('[data-testid="http-extraction"]'),
    ];
    const read = (selector: string) =>
      rows.map((row) => row.querySelector(selector)?.textContent?.trim() ?? null);
    expect(read('.path')).toEqual(['body.access_token', 'body.expires_in']);
    expect(read('.to')).toEqual(['{{accessToken}}', '{{tokenTtl}}']);
    expect(read('.secret')).toEqual(['secret', null]);
    rows[1]!.querySelector<HTMLButtonElement>('[data-testid="http-extraction-remove"]')!.click();
    expect(removed).toEqual([1]);
  });
});
