import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { Area } from '@core/services/areas/area.model';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { AreaSwitchComponent } from './area-switch.component';

describe('AreaSwitchComponent', () => {
  let fixture: ComponentFixture<AreaSwitchComponent>;
  let chosen: Area[];

  beforeEach(async () => {
    TestBed.configureTestingModule({
      imports: [AreaSwitchComponent],
      providers: [provideTranslocoTesting()],
    });
    fixture = TestBed.createComponent(AreaSwitchComponent);
    fixture.componentRef.setInput('current', 'notes');
    chosen = [];
    fixture.componentInstance.chosen.subscribe((area) => chosen.push(area));
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  const option = (area: Area): HTMLButtonElement =>
    fixture.nativeElement.querySelector(`[data-testid="area-option"][data-area="${area}"]`);

  it('names each area and marks the one showing', () => {
    expect(option('notes').getAttribute('aria-current')).toBe('page');
    expect(option('tools').getAttribute('aria-current')).toBeNull();
    expect(option('tools').textContent?.trim()).toBe('Outils');
    expect(option('tools').title).toBe('Outils (Ctrl+2)');
  });

  it('asks for another area, and says nothing for the one showing', () => {
    option('notes').click();
    option('tools').click();

    expect(chosen).toEqual(['tools']);
  });

  /** In the titlebar the icons are all that shows, and the names are still read out. */
  it('keeps the names for a screen reader in the titlebar', async () => {
    fixture.componentRef.setInput('layout', 'bar');
    await fixture.whenStable();

    expect(option('tools').querySelector('.label')?.classList).toContain('visually-hidden');
    expect(fixture.nativeElement.classList).toContain('bar');
  });
});
