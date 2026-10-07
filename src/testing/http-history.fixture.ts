import { HistoryEntry, HistoryItem } from '@core/model/http.model';
import { sentResponse } from './fake-http-repository';

/** An entry as Rust lists it: `200` unless told otherwise, sent the 7th of October at noon UTC. */
export function historyItem(id: string, overrides: Partial<HistoryItem> = {}): HistoryItem {
  return {
    id,
    sentAt: '2026-10-07T12:00:00.000Z',
    method: 'GET',
    status: 200,
    requestId: null,
    summary: { name: id, url: `https://api.exemple.fr/${id}`, millis: 142, size: 3277, failure: null },
    ...overrides,
  };
}

export function historyEntry(item: HistoryItem, answered = true): HistoryEntry {
  const response = sentResponse();
  return {
    item,
    record: { exchange: response.exchange, response: answered ? response : null },
  };
}
