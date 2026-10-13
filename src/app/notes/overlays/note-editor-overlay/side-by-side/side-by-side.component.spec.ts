import { ChangeDetectionStrategy, Component, signal } from '@angular/core';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { SideBySideComponent } from './side-by-side.component';

@Component({
  imports: [SideBySideComponent],
  template: `
    <app-side-by-side [share]="share()" (shareChanged)="share.set($event)">
      <p code class="code">code</p>
      <p explorer class="explorer">graph</p>
    </app-side-by-side>
  `,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
class HostComponent {
  readonly share = signal(40);
}

describe('SideBySideComponent', () => {
  let fixture: ComponentFixture<HostComponent>;
  const find = (selector: string) =>
    (fixture.nativeElement as HTMLElement).querySelector<HTMLElement>(selector)!;

  beforeEach(async () => {
    TestBed.configureTestingModule({ imports: [HostComponent], providers: [provideTranslocoTesting()] });
    fixture = TestBed.createComponent(HostComponent);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  it('puts the code on the left at its share, the visualiser on the right, a standing line between', () => {
    expect(find('[data-testid="side-by-side-code"]').contains(find('.code'))).toBe(true);
    expect(find('[data-testid="side-by-side-code"]').style.flexBasis).toBe('40%');
    expect(find('.explorer-pane').contains(find('.explorer'))).toBe(true);
    const line = find('[data-testid="side-by-side-handle"]');
    expect(line.getAttribute('aria-orientation')).toBe('vertical');
    expect(line.getAttribute('aria-valuetext')).toBe('Le code occupe 40 %');
  });

  it('hands a moved line back to whoever keeps the share', async () => {
    const side = (left: number, width: number) => ({ left, width, top: 0, height: 10 }) as DOMRect;
    find('app-side-by-side').getBoundingClientRect = () => side(0, 1000);
    find('[data-testid="side-by-side-code"]').getBoundingClientRect = () => side(0, 400);
    find('[data-testid="side-by-side-handle"]').dispatchEvent(
      new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true, cancelable: true }),
    );
    await fixture.whenStable();

    expect(fixture.componentInstance.share()).toBe(44);
  });
});
