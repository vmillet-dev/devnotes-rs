import { ComponentFixture, TestBed } from '@angular/core/testing';
import { By } from '@angular/platform-browser';
import { beforeEach, describe, expect, it } from 'vitest';
import { TagPillComponent } from '@notes/ui/tag-pill/tag-pill.component';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { TagRailComponent } from './tag-rail.component';

describe('TagRailComponent', () => {
  let fixture: ComponentFixture<TagRailComponent>;

  beforeEach(() => {
    TestBed.configureTestingModule({ imports: [TagRailComponent], providers: [provideTranslocoTesting()] });
    fixture = TestBed.createComponent(TagRailComponent);
    fixture.componentRef.setInput('tags', []);
    fixture.componentRef.setInput('activeTags', new Set<string>());
    fixture.autoDetectChanges();
  });

  it('renders nothing when there are no tags', () => {
    expect(fixture.debugElement.query(By.css('.tag-rail'))).toBeNull();
  });

  it('renders a tag pill per tag, marking the active ones', async () => {
    fixture.componentRef.setInput('tags', ['alpha', 'beta']);
    fixture.componentRef.setInput('activeTags', new Set(['beta']));
    await fixture.whenStable();

    const pills = fixture.debugElement
      .queryAll(By.directive(TagPillComponent))
      .map((pill) => pill.componentInstance as TagPillComponent);

    expect(pills.map((pill) => pill.label())).toEqual(['alpha', 'beta']);
    expect(pills.map((pill) => pill.active())).toEqual([false, true]);
  });

  it('groups the pills under a single accessible name', async () => {
    fixture.componentRef.setInput('tags', ['alpha']);
    await fixture.whenStable();

    const rail = fixture.nativeElement.querySelector('.tag-rail');
    expect(rail.getAttribute('role')).toBe('group');
    expect(rail.getAttribute('aria-label')).toBe('Filtrer par tag');
  });

  /**
   * Structural, because jsdom lays nothing out. Inside the scrolling row the Manage
   * button's `margin-left: auto` had no free space to claim once the tags overflowed, so
   * it followed them out of the viewport — at forty tags it sat at x=2710 in a rail 1920
   * wide. What keeps it reachable is being outside that row.
   */
  it('keeps the Manage button out of the row that scrolls', async () => {
    fixture.componentRef.setInput('tags', ['alpha', 'beta']);
    await fixture.whenStable();

    const scroll = fixture.nativeElement.querySelector('.tag-rail-scroll');
    expect(scroll.querySelectorAll('app-tag-pill')).toHaveLength(2);
    expect(scroll.querySelector('.tag-rail-manage')).toBeNull();
    expect(fixture.nativeElement.querySelector('.tag-rail > .tag-rail-manage')).not.toBeNull();
  });

  it('forwards the toggled event from a tag pill as tagToggled', async () => {
    fixture.componentRef.setInput('tags', ['alpha']);
    await fixture.whenStable();
    let emitted: string | undefined;
    fixture.componentInstance.tagToggled.subscribe((tag) => (emitted = tag));

    const pill = fixture.debugElement.query(By.directive(TagPillComponent))
      .componentInstance as TagPillComponent;
    pill.toggled.emit('alpha');

    expect(emitted).toBe('alpha');
  });
  it('offers to manage exactly the tags it displays', async () => {
    let emitted = 0;
    fixture.componentInstance.manageRequested.subscribe(() => (emitted += 1));
    fixture.componentRef.setInput('tags', ['alpha']);
    await fixture.whenStable();

    fixture.nativeElement.querySelector('.tag-rail-manage').click();
    await fixture.whenStable();

    expect(emitted).toBe(1);
  });
});
