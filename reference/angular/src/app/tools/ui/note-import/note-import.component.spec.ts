import { Component, signal } from '@angular/core';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { describe, expect, it, vi } from 'vitest';
import { StatusNotifier } from '@core/services/notifications/status.service';
import { ANY_NOTE, TOOL_CATALOGUE, ToolDefinition } from '@core/services/tools/tool.model';
import { ToolsStore } from '@core/services/tools/tools.store';
import { FakeToolComponent } from '@testing/tool-catalogue.fixture';
import { createNote } from '@testing/note.fixture';
import { provideAppTesting } from '@testing/testing.providers';
import { NoteImportComponent } from './note-import.component';
import type { PickedNote } from './note-picker/note-picker.component';

const CATALOGUE: readonly ToolDefinition[] = [
  {
    id: 'case',
    category: 'text',
    keywords: [],
    load: async () => FakeToolComponent,
    fromNote: { text: ANY_NOTE },
  },
  { id: 'jwt', category: 'crypto', keywords: [], load: async () => FakeToolComponent },
];

@Component({
  imports: [NoteImportComponent],
  template: `<app-note-import [field]="field()" (imported)="imported.push($event)" />`,
})
class HostComponent {
  readonly field = signal('text');
  readonly imported: PickedNote[] = [];
}

describe('NoteImportComponent', () => {
  async function render(toolId: string): Promise<ComponentFixture<HostComponent>> {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [HostComponent],
      providers: [
        provideAppTesting({ notes: [createNote({ id: 'readme', title: 'README', content: 'Hello' })] }),
        { provide: TOOL_CATALOGUE, useValue: CATALOGUE },
      ],
    });
    TestBed.inject(ToolsStore).open(toolId);
    const fixture = TestBed.createComponent(HostComponent);
    fixture.autoDetectChanges();
    await fixture.whenStable();
    return fixture;
  }

  const button = (fixture: ComponentFixture<HostComponent>): HTMLButtonElement | null =>
    fixture.nativeElement.querySelector('[data-testid="note-import-text"]');

  it('offers an import beside an input the open tool declares', async () => {
    const fixture = await render('case');

    expect(button(fixture)?.textContent).toContain('Importer depuis une note');
  });

  it('offers none beside an input it does not declare, nor in a tool that declares nothing', async () => {
    const fixture = await render('case');
    fixture.componentInstance.field.set('key');
    await fixture.whenStable();
    expect(fixture.nativeElement.querySelector('button')).toBeNull();

    const jwt = await render('jwt');
    expect(jwt.nativeElement.querySelector('button')).toBeNull();
  });

  it('copies the chosen note in once and says so', async () => {
    const fixture = await render('case');
    button(fixture)!.click();
    await vi.waitFor(() =>
      expect(document.querySelector('[data-testid="note-picker"] [data-note-id="readme"]')).not.toBeNull(),
    );

    document.querySelector<HTMLButtonElement>('[data-note-id="readme"]')!.click();

    await vi.waitFor(() =>
      expect(fixture.componentInstance.imported).toEqual([
        { title: 'README', content: 'Hello', language: 'txt' },
      ]),
    );
    expect(document.querySelector('[data-testid="note-picker"]')).toBeNull();
    expect(TestBed.inject(StatusNotifier).status()).toEqual({
      key: 'tools.fromNote.done',
      params: { title: 'README' },
    });
  });
});
