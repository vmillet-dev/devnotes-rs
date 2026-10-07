import { ChangeDetectionStrategy, Component, signal } from '@angular/core';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { By } from '@angular/platform-browser';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { StatusNotifier } from '@core/services/notifications/status.service';
import { Tool, ToolDefinition } from '@core/services/tools/tool.model';
import { ToolsStore } from '@core/services/tools/tools.store';
import { FAKE_TOOLS, FakeToolComponent } from '@testing/tool-catalogue.fixture';
import { provideAppTesting } from '@testing/testing.providers';
import { SaveAsNoteDialogComponent } from '@app/save-as-note/save-as-note-dialog.component';
import { ToolFrameComponent } from './tool-frame.component';

/** A tool with nothing to fill and nothing to keep, a reference: the frame offers it neither. */
@Component({ selector: 'app-bare-tool', template: '', changeDetection: ChangeDetectionStrategy.OnPush })
class BareToolComponent implements Tool {
  readonly cleared = signal(false);

  clear(): void {
    this.cleared.set(true);
  }
}

const BARE: ToolDefinition = {
  id: 'line-breaks',
  category: 'text',
  keywords: [],
  load: async () => BareToolComponent,
};

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
    expect(button('tool-title').textContent?.trim()).toBe('Hash / HMAC');
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

  it('fills the tool with its sample from the header, and says so', async () => {
    tool()!.clear();
    await fixture.whenStable();

    button('tool-sample').click();
    await fixture.whenStable();

    expect(tool()!.text()).toBe('sample');
    expect(TestBed.inject(StatusNotifier).status()).toEqual({ key: 'tools.sampleLoaded' });
  });

  it('offers no sample and nothing to keep to a tool that has neither', async () => {
    fixture.componentRef.setInput('tool', BARE);
    await vi.waitFor(() => expect(tool()).toBeNull());
    await fixture.whenStable();

    expect(button('tool-sample')).toBeNull();
    expect(button('tool-save-as-note')).toBeNull();
    expect(button('tool-clear')).not.toBeNull();
  });

  it('keeps nothing before the tool has a result, and says why', async () => {
    tool()!.clear();
    await fixture.whenStable();

    expect(button('tool-save-as-note').getAttribute('aria-disabled')).toBe('true');
    expect(button('tool-save-as-note').title).not.toBe('');
    button('tool-save-as-note').click();
    await fixture.whenStable();

    expect(fixture.debugElement.query(By.directive(SaveAsNoteDialogComponent))).toBeNull();
  });

  it('hands the dialog the result as it was on the click', async () => {
    button('tool-save-as-note').click();
    await fixture.whenStable();
    tool()!.text.set('typed afterwards');
    await fixture.whenStable();

    const dialog = fixture.debugElement.query(By.directive(SaveAsNoteDialogComponent));
    expect((dialog.componentInstance as SaveAsNoteDialogComponent).result().content).toBe('typed');
    expect((dialog.componentInstance as SaveAsNoteDialogComponent).source()).toBe('Outils / Hash / HMAC');
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
    await vi.waitFor(() => expect(tool() ?? first).not.toBe(first));

    expect(fixture.debugElement.queryAll(By.directive(FakeToolComponent))).toHaveLength(1);
  });
});
