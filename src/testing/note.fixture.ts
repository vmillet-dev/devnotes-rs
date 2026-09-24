import { ChecklistItem } from '@core/model/checklist.model';
import { Note } from '@core/model/note.model';

export function createNote(overrides: Partial<Note> = {}): Note {
  const note: Note = {
    id: 'note-1',
    spaceId: 'space-1',
    folderId: null,
    folder: null,
    title: 'Test note',
    language: 'txt',
    content: 'line one\nline two',
    source: 'Test / Fixture',
    tags: [],
    pinned: false,
    createdAt: new Date('2026-01-01T10:00:00Z'),
    updatedAt: new Date('2026-01-01T10:00:00Z'),
    lifecycle: { kind: 'permanent' },
    kind: 'snippet',
    items: [],
    // Derived by the back end; a spec about them overrides them.
    footer: { kind: 'age', at: new Date('2026-01-01T10:00:00Z') },
    expiringSoon: false,
    placeholders: [],
    attachmentCount: 0,
    copyText: null,
    searchHit: null,
    truncated: false,
    ...overrides,
  };

  return 'copyText' in overrides
    ? note
    : { ...note, copyText: note.kind === 'checklist' ? checklistMarkdown(note.items) : null };
}

/** The one back-end rule the doubles reproduce, exported so the fake does not recopy it. */
export function checklistMarkdown(items: readonly ChecklistItem[]): string {
  return items.map((item) => `- [${item.done ? 'x' : ' '}] ${item.text}`).join('\n');
}
