import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { BoardScope } from '@core/model/board.model';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { BoardTidyComponent } from './board-tidy.component';

describe('BoardTidyComponent', () => {
  let fixture: ComponentFixture<BoardTidyComponent>;
  let asked: BoardScope[];

  function button(testid: string): HTMLButtonElement | null {
    return fixture.nativeElement.querySelector(`[data-testid="${testid}"]`);
  }

  async function click(testid: string): Promise<void> {
    button(testid)?.click();
    await fixture.whenStable();
  }

  beforeEach(async () => {
    TestBed.configureTestingModule({ imports: [BoardTidyComponent], providers: [provideTranslocoTesting()] });
    fixture = TestBed.createComponent(BoardTidyComponent);
    fixture.componentRef.setInput('zoneCount', 2);
    fixture.componentRef.setInput('looseCount', 5);
    asked = [];
    fixture.componentInstance.requested.subscribe((scope) => asked.push(scope));
    document.body.appendChild(fixture.nativeElement);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  /** The cheap half in one click; the half that overwrites frames set by hand in two. */
  it('aligns the loose cards alone on the main button', async () => {
    await click('board-tidy');

    expect(asked).toEqual(['looseCards']);
  });

  it('reorganises everything only from the menu, and closes it', async () => {
    await click('board-tidy-more');
    await click('board-tidy-everything');

    expect(asked).toEqual(['everything']);
    expect(button('board-tidy-everything')).toBeNull();
  });

  it('closes its menu on Escape', async () => {
    await click('board-tidy-more');

    button('board-tidy-more')?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    await fixture.whenStable();

    expect(button('board-tidy-everything')).toBeNull();
  });
});
