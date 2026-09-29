import { ComponentFixture, TestBed } from '@angular/core/testing';
import { By } from '@angular/platform-browser';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ToolsStore } from '@core/services/tools/tools.store';
import { FAKE_TOOLS, FakeToolComponent } from '@testing/tool-catalogue.fixture';
import { provideAppTesting } from '@testing/testing.providers';
import { ToolFrameComponent } from './tool-frame.component';

describe('ToolFrameComponent', () => {
  let fixture: ComponentFixture<ToolFrameComponent>;

  beforeEach(async () => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({ imports: [ToolFrameComponent], providers: [provideAppTesting()] });
    fixture = TestBed.createComponent(ToolFrameComponent);
    fixture.componentRef.setInput('tool', FAKE_TOOLS[2]);
    fixture.autoDetectChanges();
    await vi.waitFor(() => expect(tool()).not.toBeNull());
    await fixture.whenStable();
  });

  const tool = (): FakeToolComponent | null =>
    fixture.debugElement.query(By.directive(FakeToolComponent))?.componentInstance ?? null;
  const button = (testid: string): HTMLButtonElement =>
    fixture.nativeElement.querySelector(`[data-testid="${testid}"]`);

  it('draws the tool under its breadcrumb', () => {
    expect(button('tool-title').textContent?.trim()).toBe('tools.hash.name');
    expect(fixture.nativeElement.querySelector('[data-testid="fake-tool"]').textContent).toBe('typed');
  });

  it("carries the tool's own actions, as the tool says they stand", async () => {
    button('tool-action-swap').click();
    expect(tool()!.swapped()).toBe(true);

    button('tool-clear').click();
    await fixture.whenStable();

    expect(tool()!.text()).toBe('');
    expect(button('tool-action-swap').disabled).toBe(true);
  });

  it('goes back to the home from the breadcrumb', () => {
    const store = TestBed.inject(ToolsStore);
    store.open('hash');

    button('tool-back').click();

    expect(store.openId()).toBeNull();
  });

  it('puts one tool in place of the other', async () => {
    const first = tool();
    fixture.componentRef.setInput('tool', FAKE_TOOLS[0]);
    await vi.waitFor(() => expect(tool()).not.toBe(first));

    expect(fixture.debugElement.queryAll(By.directive(FakeToolComponent))).toHaveLength(1);
  });
});
