import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { JsonLine } from '@core/model/json.model';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { JSON_LINES } from '@testing/json-view.fixture';
import { JsonTreeComponent } from './json-tree.component';

describe('JsonTreeComponent', () => {
  let fixture: ComponentFixture<JsonTreeComponent>;
  let selected: string[];
  let toggled: JsonLine[];

  beforeEach(async () => {
    TestBed.configureTestingModule({ imports: [JsonTreeComponent], providers: [provideTranslocoTesting()] });
    fixture = TestBed.createComponent(JsonTreeComponent);
    fixture.componentRef.setInput('lines', JSON_LINES);
    selected = [];
    toggled = [];
    fixture.componentInstance.selected.subscribe((path) => selected.push(path));
    fixture.componentInstance.toggled.subscribe((line) => toggled.push(line));
    document.body.appendChild(fixture.nativeElement);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  function line(path: string): HTMLElement {
    return fixture.nativeElement.querySelector(`[data-testid="json-line"][data-path="${path}"]`);
  }

  function press(key: string): void {
    fixture.nativeElement
      .querySelector('[data-testid="json-tree"]')
      .dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true }));
  }

  async function select(path: string): Promise<void> {
    fixture.componentRef.setInput('selectedPath', path);
    await fixture.whenStable();
  }

  it('indents each line by its depth and says what opens', () => {
    expect(line('$.data.lines').getAttribute('aria-level')).toBe('3');
    expect(line('$.data.lines').style.paddingLeft).toBe('48px');
    expect(line('$.data').getAttribute('aria-expanded')).toBe('true');
    expect(line('$.id').getAttribute('aria-expanded')).toBeNull();
    expect(line('$.id').textContent).toContain('"evt"');
    expect(line('$.data.lines').textContent).toContain('[ ] 1');
  });

  it('opens a container on a click, and selects a value', () => {
    line('$.data').click();
    line('$.id').click();

    expect(toggled.map((each) => each.path)).toEqual(['$.data']);
    expect(selected).toEqual(['$.id']);
  });

  it('keeps one tab stop, on the selection', async () => {
    await select('$.data');

    expect(fixture.nativeElement.querySelectorAll('[tabindex="0"]')).toHaveLength(1);
    expect(line('$.data').getAttribute('tabindex')).toBe('0');
  });

  it('walks the lines with the arrows', async () => {
    await select('$.id');

    press('ArrowDown');
    press('ArrowUp');

    expect(selected).toEqual(['$.data', '$']);
  });

  it('closes an open line on the left arrow, and goes to its parent from a closed one', async () => {
    await select('$.data');
    press('ArrowLeft');
    expect(toggled.map((each) => each.path)).toEqual(['$.data']);

    await select('$.id');
    press('ArrowLeft');
    expect(selected).toEqual(['$']);
  });

  it('opens a closed line on the right arrow, and goes into an open one', async () => {
    fixture.componentRef.setInput('lines', [JSON_LINES[0]!, { ...JSON_LINES[2]!, open: false }]);
    await select('$.data');
    press('ArrowRight');
    expect(toggled.map((each) => each.path)).toEqual(['$.data']);

    fixture.componentRef.setInput('lines', JSON_LINES);
    await select('$.data');
    press('ArrowRight');
    expect(selected).toEqual(['$.data.lines']);
  });

  it('toggles on Enter or Space, a value selected, and never closes the root', async () => {
    await select('$.data');
    line('$.data').dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    line('$.id').dispatchEvent(new KeyboardEvent('keydown', { key: ' ', bubbles: true }));
    expect(selected).toEqual(['$.id']);
    await select('$');
    press('ArrowLeft');

    expect(toggled.map((each) => each.path)).toEqual(['$.data']);
  });

  it('says so when the list was cut', async () => {
    fixture.componentRef.setInput('truncated', true);
    await fixture.whenStable();

    expect(fixture.nativeElement.querySelector('.truncated')).not.toBeNull();
  });
});
