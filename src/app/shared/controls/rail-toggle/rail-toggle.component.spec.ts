import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { RailToggleComponent } from './rail-toggle.component';

describe('RailToggleComponent', () => {
  let fixture: ComponentFixture<RailToggleComponent>;

  const button = () => (fixture.nativeElement as HTMLElement).querySelector('button')!;

  beforeEach(async () => {
    TestBed.configureTestingModule({
      imports: [RailToggleComponent],
      providers: [provideTranslocoTesting()],
    });
    fixture = TestBed.createComponent(RailToggleComponent);
    fixture.componentRef.setInput('shown', true);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  it('says whether the rail is showing, and what it does', async () => {
    expect(button().getAttribute('aria-pressed')).toBe('true');
    expect(button().getAttribute('aria-label')).toBe('Afficher ou masquer la barre latérale');

    fixture.componentRef.setInput('shown', false);
    await fixture.whenStable();

    expect(button().getAttribute('aria-pressed')).toBe('false');
  });

  it('asks, and leaves the state to whoever holds it', () => {
    let asked = 0;
    fixture.componentInstance.toggled.subscribe(() => (asked += 1));

    button().click();

    expect(asked).toBe(1);
    expect(button().getAttribute('aria-pressed')).toBe('true');
  });

  /** The notes page stays in the document behind the tools and HTTP: one test id each. */
  it('is addressed by the test id its page gives it', async () => {
    expect(button().dataset['testid']).toBe('rail-toggle');

    fixture.componentRef.setInput('testId', 'http-rail-toggle');
    await fixture.whenStable();

    expect(button().dataset['testid']).toBe('http-rail-toggle');
  });
});
