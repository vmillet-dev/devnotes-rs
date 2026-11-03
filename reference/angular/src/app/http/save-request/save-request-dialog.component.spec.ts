import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { HttpCollectionsStore } from '@core/services/http/http-collections.store';
import { FakeHttpRepository } from '@testing/fake-http-repository';
import { sampleTree } from '@testing/http-tree.fixture';
import { provideAppTesting } from '@testing/testing.providers';
import { SaveAnswer, SaveRequestDialogComponent } from './save-request-dialog.component';

/**
 * On a library with no collection the dialog answered « créez-en une dans le rail » and
 * could only be cancelled: saving a first request took a trip to the rail and back.
 */
describe('SaveRequestDialogComponent', () => {
  let fixture: ComponentFixture<SaveRequestDialogComponent>;
  let http: FakeHttpRepository;
  let saved: SaveAnswer[];

  const el = <E extends HTMLElement = HTMLElement>(selector: string): E =>
    document.body.querySelector(selector) as E;
  const field = (testid: string) => el<HTMLInputElement>(`[data-testid="${testid}"]`);
  const options = () =>
    [...document.body.querySelectorAll<HTMLElement>('[data-testid="choice-option"]')].map(
      (option) => option.dataset['optionId'],
    );

  async function open(collections = true): Promise<void> {
    TestBed.resetTestingModule();
    http = new FakeHttpRepository();
    http.tree$ = collections ? sampleTree() : { collections: [] };
    TestBed.configureTestingModule({
      imports: [SaveRequestDialogComponent],
      providers: [provideAppTesting({ httpRepository: http })],
    });
    await TestBed.inject(HttpCollectionsStore).load();
    fixture = TestBed.createComponent(SaveRequestDialogComponent);
    fixture.componentRef.setInput('name', 'Nouvelle requête');
    saved = [];
    fixture.componentInstance.saved.subscribe((answer) => saved.push(answer));
    fixture.autoDetectChanges();
    await fixture.whenStable();
  }

  async function choose(placeId: string): Promise<void> {
    field('http-save-place').click();
    await fixture.whenStable();
    el<HTMLButtonElement>(`[data-testid="choice-option"][data-option-id="${placeId}"]`).click();
    await fixture.whenStable();
  }

  async function submit(): Promise<void> {
    el<HTMLButtonElement>('[data-testid="http-save-submit"]').click();
    await fixture.whenStable();
  }

  beforeEach(() => {
    document.body.innerHTML = '';
  });

  describe('on a library with no collection', () => {
    beforeEach(() => open(false));

    it('asks for the name of the collection it will create, in place of a menu with nothing in it', () => {
      expect(el('[data-testid="http-save-place"]')).toBeNull();
      expect(field('http-save-collection').placeholder).toBe('Nom de la nouvelle collection');
      // No folder to offer under a collection that is not there yet.
      expect(el('[data-testid="http-save-new-folder"]')).toBeNull();
      expect(el<HTMLButtonElement>('[data-testid="http-save-submit"]').disabled).toBe(false);
    });

    it('answers the request and the collection to create with it: one gesture', async () => {
      field('http-save-name').value = '  Santé ';
      field('http-save-collection').value = '  Boutique ';
      await submit();

      expect(saved).toEqual([
        { collectionId: '', folderId: null, name: 'Santé', create: { collection: 'Boutique' } },
      ]);
    });

    it('refuses a collection without a name in place: the field is marked, focused, and says so', async () => {
      await submit();

      expect(saved).toEqual([]);
      expect(el('[data-testid="http-save-missing"]').textContent?.trim()).toBe(
        'Donnez un nom à la collection.',
      );
      expect(field('http-save-collection').getAttribute('aria-invalid')).toBe('true');
      expect(field('http-save-collection').getAttribute('aria-describedby')).toBe('http-save-missing');
      expect(document.activeElement).toBe(field('http-save-collection'));

      // Typing is the answer: the sentence goes.
      field('http-save-collection').value = 'B';
      field('http-save-collection').dispatchEvent(new Event('input', { bubbles: true }));
      await fixture.whenStable();
      expect(el('[data-testid="http-save-missing"]')).toBeNull();
      expect(field('http-save-collection').getAttribute('aria-invalid')).toBeNull();
    });
  });

  describe('on a library that holds collections', () => {
    beforeEach(() => open());

    it('saves where it is told, as before, and refuses a request without a name', async () => {
      await submit();
      expect(saved).toEqual([{ collectionId: 'API', folderId: null, name: 'Nouvelle requête' }]);

      field('http-save-name').value = '   ';
      await submit();
      expect(saved).toHaveLength(1);
      expect(el('[data-testid="http-save-missing"]').textContent?.trim()).toBe('Donnez un nom à la requête.');
      expect(document.activeElement).toBe(field('http-save-name'));
    });

    it('ends the menu of places with a new collection, which turns into a name to type', async () => {
      field('http-save-place').click();
      await fixture.whenStable();
      expect(options().at(-1)).toBe('new-collection');
      expect(options().slice(0, 2)).toEqual(['API', 'Auth']);
      expect(el('[data-testid="choice-option"][data-option-id="new-collection"]').textContent?.trim()).toBe(
        'Nouvelle collection…',
      );

      el<HTMLButtonElement>('[data-testid="choice-option"][data-option-id="new-collection"]').click();
      await fixture.whenStable();
      await vi.waitFor(() => expect(document.activeElement).toBe(field('http-save-collection')));
      expect(el('[data-testid="http-save-new-folder"]')).toBeNull();

      field('http-save-collection').value = 'Catalogue';
      await submit();
      expect(saved).toEqual([
        { collectionId: '', folderId: null, name: 'Nouvelle requête', create: { collection: 'Catalogue' } },
      ]);
    });

    it('offers a new folder under the place chosen, a collection or a folder', async () => {
      await choose('Auth');
      el<HTMLButtonElement>('[data-testid="http-save-new-folder"]').click();
      await fixture.whenStable();
      await vi.waitFor(() => expect(document.activeElement).toBe(field('http-save-folder')));
      expect(field('http-save-folder').placeholder).toBe('Nom du nouveau dossier');

      field('http-save-folder').value = ' Jetons ';
      await submit();
      expect(saved).toEqual([
        { collectionId: 'API', folderId: 'Auth', name: 'Nouvelle requête', create: { folder: 'Jetons' } },
      ]);
    });

    it('refuses a folder asked for and left without a name, until it is named or taken back', async () => {
      el<HTMLButtonElement>('[data-testid="http-save-new-folder"]').click();
      await fixture.whenStable();
      await submit();
      expect(saved).toEqual([]);
      expect(el('[data-testid="http-save-missing"]').textContent?.trim()).toBe(
        'Donnez un nom au dossier, ou retirez-le.',
      );
      expect(field('http-save-folder').getAttribute('aria-invalid')).toBe('true');

      el<HTMLButtonElement>('[data-testid="http-save-folder-cancel"]').click();
      await fixture.whenStable();
      expect(el('[data-testid="http-save-folder"]')).toBeNull();
      expect(el('[data-testid="http-save-missing"]')).toBeNull();
      await submit();
      expect(saved).toEqual([{ collectionId: 'API', folderId: null, name: 'Nouvelle requête' }]);
    });

    it('asks for the folder again when another place is chosen', async () => {
      el<HTMLButtonElement>('[data-testid="http-save-new-folder"]').click();
      await fixture.whenStable();
      expect(el('[data-testid="http-save-folder"]')).not.toBeNull();

      await choose('Catalogue');
      expect(el('[data-testid="http-save-folder"]')).toBeNull();
      expect(el('[data-testid="http-save-new-folder"]')).not.toBeNull();
    });

    it('is left without writing from its cancel button', () => {
      let cancelled = 0;
      fixture.componentInstance.cancelled.subscribe(() => (cancelled += 1));
      el<HTMLButtonElement>('[data-testid="http-save-cancel"]').click();
      expect(cancelled).toBe(1);
      expect(saved).toEqual([]);
    });
  });
});
