import { LibrariesRepository, LibraryEntry, Registry } from '@core/data/libraries.repository';
import { IpcError } from '@core/ipc/ipc.error';

function entry(id: string, name: string): LibraryEntry {
  return {
    id,
    name,
    directory: `libraries/${id}`,
    createdAt: '2026-07-25T09:00:00.000Z',
  };
}

/** The real one reaches for the Tauri bridge, absent under jsdom. */
export class FakeLibrariesRepository implements Pick<LibrariesRepository, keyof LibrariesRepository> {
  failNext: IpcError | null = null;

  private registry: Registry;
  private next = 0;

  constructor(names: readonly string[] = ['Notes']) {
    this.registry = {
      libraries: names.map((name, index) => entry(`lib-${index}`, name)),
      open: names.length > 0 ? 'lib-0' : null,
    };
    this.next = names.length;
  }

  async list(): Promise<Registry> {
    this.check();

    return { libraries: [...this.registry.libraries], open: this.registry.open };
  }

  async create(name: string): Promise<LibraryEntry> {
    this.check();
    const created = entry(`lib-${this.next++}`, name);
    this.registry.libraries.push(created);

    return created;
  }

  async open(id: string): Promise<void> {
    this.check();
    this.registry.open = id;
  }

  async rename(id: string, name: string): Promise<void> {
    this.check();
    this.registry.libraries = this.registry.libraries.map((each) =>
      each.id === id ? { ...each, name } : each,
    );
  }

  async delete(id: string): Promise<void> {
    this.check();
    this.registry.libraries = this.registry.libraries.filter((each) => each.id !== id);
    if (this.registry.open === id) {
      this.registry.open = this.registry.libraries[0]?.id ?? null;
    }
  }

  private check(): void {
    const failure = this.failNext;
    this.failNext = null;
    if (failure !== null) throw failure;
  }
}
