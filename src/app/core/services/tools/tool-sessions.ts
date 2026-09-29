import { Injectable, WritableSignal, inject, signal } from '@angular/core';

/**
 * What each tool held, for the session: a tool left and found again is as it was. In memory
 * alone, never on disk, and gone with the page when the library changes. ⚠️ No secret goes
 * here — a key or a password is a plain `signal` of the tool, which dies with it.
 */
@Injectable({ providedIn: 'root' })
export class ToolSessions {
  private readonly slots = new Map<string, WritableSignal<unknown>>();

  slot<T>(key: string, initial: T): WritableSignal<T> {
    let slot = this.slots.get(key);
    if (slot === undefined) {
      slot = signal<unknown>(initial);
      this.slots.set(key, slot);
    }
    return slot as WritableSignal<T>;
  }
}

/** In a tool's injection context, keyed by the tool: `toolState('slug.text', '')`. */
export function toolState<T>(key: string, initial: T): WritableSignal<T> {
  return inject(ToolSessions).slot(key, initial);
}
