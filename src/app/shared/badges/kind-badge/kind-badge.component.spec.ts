import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { KindBadgeComponent } from './kind-badge.component';

describe('KindBadgeComponent', () => {
  let fixture: ComponentFixture<KindBadgeComponent>;

  beforeEach(() => {
    TestBed.configureTestingModule({ imports: [KindBadgeComponent], providers: [provideTranslocoTesting()] });
    fixture = TestBed.createComponent(KindBadgeComponent);
    fixture.componentRef.setInput('language', 'sql');
  });

  async function text(kind: 'snippet' | 'note' | 'checklist'): Promise<string> {
    fixture.componentRef.setInput('kind', kind);
    await fixture.whenStable();
    return (fixture.nativeElement as HTMLElement).textContent?.trim() ?? '';
  }

  it('names a snippet by its format', async () => {
    expect(await text('snippet')).toBe('SQL');
  });

  it('names a Note and a list by what they are, whatever their stored language', async () => {
    expect(await text('note')).toBe('Note');
    expect(await text('checklist')).toBe('✓ Tâches');
  });
});
