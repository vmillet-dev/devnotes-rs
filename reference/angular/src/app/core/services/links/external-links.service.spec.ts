import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { EXTERNAL_LINK_ADAPTER, ExternalLinksService } from './external-links.service';

describe('ExternalLinksService', () => {
  let opened: string[];
  let service: ExternalLinksService;

  beforeEach(() => {
    opened = [];
    TestBed.configureTestingModule({
      providers: [
        { provide: EXTERNAL_LINK_ADAPTER, useValue: { open: async (url: string) => void opened.push(url) } },
      ],
    });
    service = TestBed.inject(ExternalLinksService);
  });

  it('opens a web address in the browser', async () => {
    expect(await service.open('https://example.com/runbook')).toBe(true);
    expect(opened).toEqual(['https://example.com/runbook']);
  });

  it('refuses anything that is not the web', async () => {
    for (const url of ['file:///C:/Windows/System32/calc.exe', 'javascript:alert(1)', 'ftp://host']) {
      expect(await service.open(url)).toBe(false);
    }
    expect(opened).toEqual([]);
  });
});
