import { describe, expect, it } from 'vitest';
import type { DisplayNote as WireNote } from '@core/ipc/bindings';
import { Note, NoteDraft } from '@core/model/note.model';
import {
  ContractError,
  toAttachment,
  toNote,
  toWireNoteDraft,
  toWireNotePatch,
  toTrashedNote,
} from './note.mapper';

const BASE_DTO: WireNote = {
  id: 'note-1',
  spaceId: 'space-1',
  title: 'Payload',
  language: 'json',
  content: '{}',
  source: 'API',
  tags: ['a'],
  pinned: false,
  createdAt: '2026-01-01T10:00:00.000Z',
  updatedAt: '2026-01-02T10:00:00.000Z',
  lifecycle: { kind: 'permanent' },
  footer: { kind: 'age', at: '2026-01-02T10:00:00.000Z' },
  expiringSoon: false,
  placeholders: [],
  attachmentCount: 0,
  folder: null,
  copyText: null,
  searchHit: null,
  truncated: false,
};

describe('toNote', () => {
  it('parses ISO date strings into Date instances', () => {
    const note = toNote(BASE_DTO);

    expect(note.createdAt).toBeInstanceOf(Date);
    expect(note.createdAt.toISOString()).toBe('2026-01-01T10:00:00.000Z');
    expect(note.updatedAt.toISOString()).toBe('2026-01-02T10:00:00.000Z');
  });

  it('parses the expiry date of an expiring lifecycle', () => {
    const note = toNote({ ...BASE_DTO, lifecycle: { kind: 'expires', at: '2026-03-01T00:00:00.000Z' } });

    expect(note.lifecycle).toEqual({ kind: 'expires', at: new Date('2026-03-01T00:00:00.000Z') });
  });

  it('copies the remaining fields verbatim', () => {
    const note = toNote(BASE_DTO);

    expect(note).toMatchObject({
      id: 'note-1',
      spaceId: 'space-1',
      title: 'Payload',
      language: 'json',
      content: '{}',
      source: 'API',
      tags: ['a'],
      pinned: false,
    });
  });

  it('takes the language straight from the wire, with no narrowing left to do', () => {
    const note = toNote({ ...BASE_DTO, language: 'sql' });

    expect(note.language).toBe('sql');
  });

  it('throws a contract error on an unparseable date rather than yielding an Invalid Date', () => {
    expect(() => toNote({ ...BASE_DTO, createdAt: 'not-a-date' })).toThrow(ContractError);
  });

  it('throws a contract error on an unparseable expiry date', () => {
    expect(() => toNote({ ...BASE_DTO, lifecycle: { kind: 'expires', at: 'nope' } })).toThrow(ContractError);
  });

  describe('footer', () => {
    it('parses the date of a dated footer so the label can age on screen', () => {
      const note = toNote({ ...BASE_DTO, footer: { kind: 'age', at: '2026-01-02T10:00:00.000Z' } });

      expect(note.footer).toEqual({ kind: 'age', at: new Date('2026-01-02T10:00:00.000Z') });
    });

    it('carries an expiry footer as its own variant, formatted differently', () => {
      const note = toNote({ ...BASE_DTO, footer: { kind: 'expiry', at: '2026-03-01T00:00:00.000Z' } });

      expect(note.footer).toEqual({ kind: 'expiry', at: new Date('2026-03-01T00:00:00.000Z') });
    });

    it('carries a source footer as plain text, with no date to parse', () => {
      const note = toNote({ ...BASE_DTO, footer: { kind: 'source', value: 'API Gateway' } });

      expect(note.footer).toEqual({ kind: 'source', value: 'API Gateway' });
    });

    it('throws a contract error on a footer variant this build does not know', () => {
      const unknown = { ...BASE_DTO, footer: { kind: 'weather', at: '2026-01-01' } } as unknown as WireNote;

      expect(() => toNote(unknown)).toThrow(ContractError);
    });

    it('carries the expiry proximity the backend decided', () => {
      expect(toNote({ ...BASE_DTO, expiringSoon: true }).expiringSoon).toBe(true);
    });
  });
});

describe('toWireNoteDraft', () => {
  it('serialises dates and omits the fields the backend owns', () => {
    const draft: NoteDraft = {
      spaceId: 'space-1',
      folderId: null,
      title: 'New',
      language: 'txt',
      content: '',
      source: '',
      tags: [],
      pinned: false,
      lifecycle: { kind: 'expires', at: new Date('2026-05-01T00:00:00.000Z') },
      kind: 'snippet',
      items: [],
    };

    const dto = toWireNoteDraft(draft);

    expect(dto).toEqual({
      spaceId: 'space-1',
      folderId: null,
      title: 'New',
      language: 'txt',
      content: '',
      source: '',
      tags: [],
      pinned: false,
      lifecycle: { kind: 'expires', at: '2026-05-01T00:00:00.000Z' },
      kind: 'snippet',
      items: [],
    });
    expect(dto).not.toHaveProperty('id');
    expect(dto).not.toHaveProperty('createdAt');
  });
});

describe('toWireNotePatch', () => {
  it('isolates the checklist it sends from the array it was given', () => {
    const outgoing = [{ text: 'Relire', done: true }];

    const dto = toWireNotePatch({ items: outgoing });
    outgoing[0].text = 'Changed afterwards';

    expect(dto.items).toEqual([{ text: 'Relire', done: true }]);
  });

  it('includes only the fields actually present in the patch', () => {
    const dto = toWireNotePatch({ pinned: true });

    expect(dto).toEqual({ pinned: true });
    expect(Object.keys(dto)).toEqual(['pinned']);
  });

  it('keeps falsy values that were explicitly set', () => {
    const dto = toWireNotePatch({ title: '', pinned: false });

    expect(dto).toEqual({ title: '', pinned: false });
  });

  it('carries a space change, which is how a note is moved between spaces', () => {
    expect(toWireNotePatch({ spaceId: 'space-2' })).toEqual({ spaceId: 'space-2' });
  });

  it('serialises a lifecycle change', () => {
    const patch: Partial<Note> = { lifecycle: { kind: 'expires', at: new Date('2026-06-01T00:00:00.000Z') } };

    expect(toWireNotePatch(patch)).toEqual({
      lifecycle: { kind: 'expires', at: '2026-06-01T00:00:00.000Z' },
    });
  });

  it('produces an empty object for an empty patch', () => {
    expect(toWireNotePatch({})).toEqual({});
  });
});

describe('toTrashedNote', () => {
  const DTO = {
    id: 'note-1',
    spaceId: 'space-1',
    title: 'Deleted',
    language: 'sql' as const,
    content: 'select 1',
    source: '',
    tags: ['auth'],
    pinned: false,
    createdAt: '2026-08-01T10:00:00.000Z',
    updatedAt: '2026-08-02T10:00:00.000Z',
    lifecycle: { kind: 'permanent' as const },
    deletedAt: '2026-08-27T08:00:00.000Z',
    purgeAt: '2026-09-26T08:00:00.000Z',
  };

  it('parses both trash dates', () => {
    const note = toTrashedNote(DTO);

    expect(note.deletedAt.toISOString()).toBe('2026-08-27T08:00:00.000Z');
    expect(note.purgeAt.toISOString()).toBe('2026-09-26T08:00:00.000Z');
  });

  it('carries only what the panel shows', () => {
    const note = toTrashedNote(DTO);

    expect(note).toEqual({
      id: 'note-1',
      spaceId: 'space-1',
      title: 'Deleted',
      language: 'sql',
      content: 'select 1',
      tags: ['auth'],
      deletedAt: new Date('2026-08-27T08:00:00.000Z'),
      purgeAt: new Date('2026-09-26T08:00:00.000Z'),
      kind: 'snippet',
    });
  });

  it('copies the tags rather than aliasing the payload', () => {
    const note = toTrashedNote(DTO);

    expect(note.tags).not.toBe(DTO.tags);
  });

  it('fails loudly on an unreadable date', () => {
    expect(() => toTrashedNote({ ...DTO, purgeAt: 'jamais' })).toThrow(ContractError);
  });
});

describe('toAttachment', () => {
  it('parses the creation instant and copies the rest verbatim', () => {
    const attachment = toAttachment({
      id: 'attachment-1',
      noteId: 'note-1',
      fileName: 'capture.png',
      mimeType: 'image/png',
      byteSize: 2048,
      createdAt: '2026-08-27T09:00:00.000Z',
    });

    expect(attachment.createdAt).toBeInstanceOf(Date);
    expect(attachment.fileName).toBe('capture.png');
    expect(attachment.byteSize).toBe(2048);
  });
});
