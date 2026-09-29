import { Type } from '@angular/core';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { vi } from 'vitest';
import { FakeClipboard } from './fake-clipboard';
import { FakeToolsRepository } from './fake-tools-repository';
import { provideAppTesting } from './testing.providers';

export interface ToolHarness<T> {
  readonly fixture: ComponentFixture<T>;
  readonly tool: T;
  readonly tools: FakeToolsRepository;
  readonly clipboard: FakeClipboard;
  /** Types into a field as a user would, and waits past the debounce for Rust to be asked. */
  type(testid: string, text: string, command: string): Promise<void>;
  element<E extends HTMLElement = HTMLElement>(selector: string): E;
  all(selector: string): HTMLElement[];
  settle(): Promise<void>;
}

/** A tool on its own, its Rust answers set on `tools` before the first request. */
export async function renderTool<T>(
  component: Type<T>,
  prepare: (tools: FakeToolsRepository) => void = () => undefined,
): Promise<ToolHarness<T>> {
  TestBed.resetTestingModule();
  const tools = new FakeToolsRepository();
  const clipboard = new FakeClipboard();
  prepare(tools);
  TestBed.configureTestingModule({
    imports: [component],
    providers: [provideAppTesting({ toolsRepository: tools, clipboard })],
  });
  const fixture = TestBed.createComponent(component);
  fixture.autoDetectChanges();
  await fixture.whenStable();

  const element = <E extends HTMLElement>(selector: string): E =>
    fixture.nativeElement.querySelector(selector);

  return {
    fixture,
    tool: fixture.componentInstance,
    tools,
    clipboard,
    element,
    all: (selector) => [...fixture.nativeElement.querySelectorAll(selector)],
    settle: () => fixture.whenStable(),
    async type(testid, text, command) {
      const before = tools.requestsOf(command).length;
      const field = element<HTMLInputElement | HTMLTextAreaElement>(`[data-testid="${testid}"]`);
      field.value = text;
      field.dispatchEvent(new Event('input'));
      await vi.waitFor(() => {
        if (tools.requestsOf(command).length === before) throw new Error(`${command} was not asked`);
      });
      await fixture.whenStable();
    },
  };
}
