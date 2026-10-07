import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { HttpCollectionsStore } from '@core/services/http/http-collections.store';
import { SettingsStore } from '@core/services/settings/settings.store';
import { FakeHttpRepository } from '@testing/fake-http-repository';
import { sampleTree } from '@testing/http-tree.fixture';
import { provideAppTesting } from '@testing/testing.providers';
import { HttpPageComponent } from './http-page.component';

describe('HttpPageComponent', () => {
  let fixture: ComponentFixture<HttpPageComponent>;
  let http: FakeHttpRepository;

  beforeEach(async () => {
    TestBed.resetTestingModule();
    http = new FakeHttpRepository();
    http.tree$ = sampleTree();
    TestBed.configureTestingModule({
      imports: [HttpPageComponent],
      providers: [provideAppTesting({ httpRepository: http })],
    });
    fixture = TestBed.createComponent(HttpPageComponent);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  const el = (selector: string): HTMLElement | null => fixture.nativeElement.querySelector(selector);

  it('reads the collections when it opens, and says how to begin while nothing is open', async () => {
    await vi.waitFor(() => expect(http.callsOf('tree').length).toBeGreaterThan(0));

    expect(el('[data-testid="http-rail"]')).not.toBeNull();
    expect(el('[data-testid="http-workspace-empty"]')).not.toBeNull();
  });

  it('shows the open request, read by its id, its URL or that it has none', async () => {
    const created = await http.createRequest({
      collectionId: 'API',
      folderId: null,
      name: 'Lister',
      kind: 'graphql',
      method: 'POST',
      document: { url: '', description: '' },
    });
    TestBed.inject(HttpCollectionsStore).open(created.id);

    await vi.waitFor(() => expect(el('[data-testid="http-open-request"]')).not.toBeNull());
    expect(el('[data-testid="http-open-request"] .method')?.textContent).toBe('QUERY');
    expect(el('[data-testid="http-open-request"] h1')?.textContent).toBe('Lister');
    expect(el('[data-testid="http-open-url"]')?.textContent?.trim()).toBe("Pas encore d'URL");
  });

  it('hides and shows its rail on Ctrl+B, as the notes do', async () => {
    const settings = TestBed.inject(SettingsStore);
    const shown = settings.showLibraryRail();

    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'b', ctrlKey: true }));
    await fixture.whenStable();

    expect(settings.showLibraryRail()).toBe(!shown);
    expect(el('[data-testid="http-rail"]') === null).toBe(shown);
  });
});
