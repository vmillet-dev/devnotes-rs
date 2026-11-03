import { TestBed } from '@angular/core/testing';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { NotesRepository } from '@core/data/notes.repository';
import { FakeNotesRepository } from '@testing/fake-notes-repository';
import { provideAppTesting } from '@testing/testing.providers';
import { FIELD_MARKS_UP_TO, FieldMarksStore } from './field-marks.store';

describe('FieldMarksStore', () => {
  let store: FieldMarksStore;
  let repository: FakeNotesRepository;

  beforeEach(() => {
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    TestBed.resetTestingModule();
    repository = new FakeNotesRepository();
    TestBed.configureTestingModule({ providers: [provideAppTesting({ notesRepository: repository })] });
    store = TestBed.inject(FieldMarksStore);
    repository.spans = [{ from: 0, to: 8, default: null }];
  });

  afterEach(() => vi.useRealTimers());

  it('asks once the typing pauses, and keeps the text the answer was found in', async () => {
    const ask = vi.spyOn(TestBed.inject(NotesRepository), 'fieldSpans');

    store.follow('{{h');
    store.follow('{{host}}');
    await vi.runAllTimersAsync();

    expect(ask).toHaveBeenCalledTimes(1);
    expect(store.marks()).toEqual({ text: '{{host}}', spans: [{ from: 0, to: 8, default: null }] });
  });

  it('drops an answer for a text that has moved on', async () => {
    let answer: (value: never[]) => void = () => undefined;
    vi.spyOn(repository, 'fieldSpans').mockReturnValueOnce(new Promise((resolve) => (answer = resolve)));

    store.follow('{{a}}');
    await vi.runAllTimersAsync();
    store.follow('{{a}} {{b}}');
    answer([]);
    await vi.runAllTimersAsync();

    expect(store.marks()?.text).toBe('{{a}} {{b}}');
  });

  it('marks nothing past the limit, nor outside a snippet', async () => {
    store.follow('{{host}}');
    await vi.runAllTimersAsync();

    store.follow('x'.repeat(FIELD_MARKS_UP_TO + 1));
    expect(store.marks()).toBeNull();

    store.follow(null);
    await vi.runAllTimersAsync();
    expect(store.marks()).toBeNull();
  });

  it('leaves the code unmarked when the answer fails, saying nothing', async () => {
    repository.failNext = new Error('gone');

    store.follow('{{host}}');
    await vi.runAllTimersAsync();

    expect(store.marks()).toEqual({ text: '{{host}}', spans: [] });
  });
});
