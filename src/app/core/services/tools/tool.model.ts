import { InjectionToken, Signal, Type } from '@angular/core';
import { LanguageTag } from '@core/model/language.model';
import { NoteKind } from '@core/model/note.model';
import { TranslationRef } from '@core/services/i18n/translation-ref.model';

/** In the home's order, which the rail follows. */
export const TOOL_CATEGORIES = ['text', 'crypto', 'encode', 'compare', 'generate'] as const;
export type ToolCategory = (typeof TOOL_CATEGORIES)[number];

/** Its name and its line are the keys `tools.<id>.name` and `tools.<id>.description`. */
export interface ToolDefinition {
  readonly id: string;
  readonly category: ToolCategory;
  /** What a search also finds it by, in no language in particular: `md5`, `camelCase`. */
  readonly keywords: readonly string[];
  readonly load: () => Promise<Type<Tool>>;
}

/** What "Enregistrer comme note" keeps: the tool decides, so the dialog asks nothing it could infer. */
export interface ToolResult {
  /** Proposed, and the dialog lets it be changed. */
  readonly title: TranslationRef;
  readonly kind: NoteKind;
  readonly language: LanguageTag;
  readonly content: string;
}

/** A button of the tool's own in the frame's header: "Échanger A et B". */
export interface ToolAction {
  readonly id: string;
  readonly labelKey: string;
  readonly disabled: boolean;
  readonly run: () => void;
}

/**
 * All the frame knows of a tool. ⚠️ `result` is an output, never an input: an HMAC key or a
 * password typed into a tool cannot reach a note, because nothing here carries one.
 */
export interface Tool {
  readonly result: Signal<ToolResult | null>;
  readonly actions?: Signal<readonly ToolAction[]>;
  clear(): void;
}

/** Provided by the tools page, so the home, the rail and the frame can be tested on a fake one. */
export const TOOL_CATALOGUE = new InjectionToken<readonly ToolDefinition[]>('TOOL_CATALOGUE');

/** Which tool was opened, and when: nothing of what was typed in it. */
export interface RecentTool {
  readonly id: string;
  readonly at: string;
}

export const MAX_RECENT_TOOLS = 5;

export function isRecentTool(value: unknown): value is RecentTool {
  const candidate = value as Partial<RecentTool> | null;
  return typeof candidate?.id === 'string' && typeof candidate.at === 'string';
}

/** Case and accents aside, as the notes' search does in Rust: "hachage" finds "Hachage". */
export function foldForSearch(text: string): string {
  return text.normalize('NFD').replace(/\p{M}/gu, '').toLowerCase();
}

export function toolMatches(query: string, texts: readonly string[]): boolean {
  const wanted = foldForSearch(query.trim());
  return wanted === '' || texts.some((text) => foldForSearch(text).includes(wanted));
}
