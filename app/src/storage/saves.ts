import type { SaveGame, SaveSummary } from '../api';

/** A stored save, as the load screen lists it. */
export interface SaveSlot {
  name: string;
  savedAt: number;
  summary: SaveSummary;
}

/**
 * Where saves live. The browser keeps them in IndexedDB; a native shell
 * could keep them as files behind the same interface.
 */
export interface SaveStore {
  list(): Promise<SaveSlot[]>;
  put(name: string, save: SaveGame): Promise<void>;
  get(name: string): Promise<SaveGame | null>;
  remove(name: string): Promise<void>;
}

type Row = SaveSlot & { data: string };

const DB = 'footballsim';
const STORE = 'saves';

function request<T>(r: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    r.onsuccess = () => resolve(r.result);
    r.onerror = () => reject(r.error);
  });
}

/** Saves in IndexedDB. Safari on iPad keeps these for installed web apps. */
export class IndexedDbSaveStore implements SaveStore {
  private db: Promise<IDBDatabase> | null = null;

  private open(): Promise<IDBDatabase> {
    this.db ??= new Promise((resolve, reject) => {
      const r = indexedDB.open(DB, 1);
      r.onupgradeneeded = () => r.result.createObjectStore(STORE, { keyPath: 'name' });
      r.onsuccess = () => resolve(r.result);
      r.onerror = () => reject(r.error);
    });
    return this.db;
  }

  private async store(mode: IDBTransactionMode): Promise<IDBObjectStore> {
    return (await this.open()).transaction(STORE, mode).objectStore(STORE);
  }

  async list(): Promise<SaveSlot[]> {
    const rows = await request((await this.store('readonly')).getAll() as IDBRequest<Row[]>);
    return rows
      .map(({ name, savedAt, summary }) => ({ name, savedAt, summary }))
      .sort((a, b) => b.savedAt - a.savedAt);
  }

  async put(name: string, save: SaveGame): Promise<void> {
    const row: Row = { name, savedAt: Date.now(), summary: save.summary, data: save.data };
    await request((await this.store('readwrite')).put(row));
  }

  async get(name: string): Promise<SaveGame | null> {
    const row = await request((await this.store('readonly')).get(name) as IDBRequest<Row | undefined>);
    return row ? { data: row.data, summary: row.summary } : null;
  }

  async remove(name: string): Promise<void> {
    await request((await this.store('readwrite')).delete(name));
  }
}

/** Saves in memory, for tests and private browsing. */
export class MemorySaveStore implements SaveStore {
  private rows = new Map<string, Row>();

  async list(): Promise<SaveSlot[]> {
    return [...this.rows.values()]
      .map(({ name, savedAt, summary }) => ({ name, savedAt, summary }))
      .sort((a, b) => b.savedAt - a.savedAt);
  }

  async put(name: string, save: SaveGame): Promise<void> {
    this.rows.set(name, { name, savedAt: Date.now(), summary: save.summary, data: save.data });
  }

  async get(name: string): Promise<SaveGame | null> {
    const row = this.rows.get(name);
    return row ? { data: row.data, summary: row.summary } : null;
  }

  async remove(name: string): Promise<void> {
    this.rows.delete(name);
  }
}

/** IndexedDB when available, otherwise memory. */
export function defaultSaveStore(): SaveStore {
  return typeof indexedDB === 'undefined' ? new MemorySaveStore() : new IndexedDbSaveStore();
}

/** The slot the game saves to automatically. */
export const AUTOSAVE = 'Autosave';
