import { $, $$, browser } from '@wdio/globals';

import type { GlobalAction } from '@core/ipc/bindings';

/** `[data-testid="…"]`, in one place, so a spec never spells the attribute. */
export function testid(name: string): string {
  return `[data-testid="${name}"]`;
}

/** Consecutive quiet polls before the canvas counts as settled, `SETTLE_INTERVAL` apart. */
const SETTLE_POLLS = 3;
const SETTLE_INTERVAL = 200;

/**
 * The canvas is behind a lazy route, a `resource` and a debounce; every scenario waits
 * for it rather than for the window, which exists long before anything is queryable.
 */
/**
 * ⚠️ The library is encrypted, so a run creates this on its first launch and carries it
 * for the rest. There is nothing to remember between runs: `resetProfile` deletes the key
 * file with everything else, so every run starts from a library that has none.
 */
export const PASSPHRASE = 'an end-to-end passphrase';

/**
 * Passes the unlock screen when it is there, and says nothing when it is not.
 *
 * ⚠️ Only `01-first-launch` ever meets it. The unlocked state lives in Rust and the
 * process outlives every page reload — `before()` and `reopenSession` both rebuild the
 * front end over a library that is already open — so a later file finds no gate at all.
 */
export async function passTheGate(): Promise<void> {
  const field = $(testid('vault-passphrase'));
  if (!(await field.isExisting())) return;

  await setField(testid('vault-passphrase'), PASSPHRASE);

  // Two entries on a library that has never been encrypted, one on a locked one.
  if (await $(testid('vault-confirmation')).isExisting()) {
    await setField(testid('vault-confirmation'), PASSPHRASE);
  }

  await $(testid('vault-submit')).click();
  // ⚠️ Generous: deriving the key is deliberately slow, and a first launch seals whatever
  // was already there on top of it.
  await field.waitForExist({ reverse: true, timeout: 60_000 });
}

export async function waitForCanvas(): Promise<void> {
  await passTheGate();
  await $(testid('canvas')).waitForExist({ timeout: 30_000 });

  // ⚠️ `aria-busy` answers "has anything at all arrived yet", once, and then says nothing
  // ever again — the retained `linkedSignal` keeps `isLoading` false for the rest of the
  // session. It gives no signal for the reload that follows a write, and the canvas
  // re-renders under the spec either way.
  //
  // So settled means idle *and* unchanged: the card count has to hold still. It
  // deliberately does not wait for a number, so it cannot assume the answer a spec is
  // about to assert.
  let previous = -1;
  let quiet = 0;

  await browser.waitUntil(
    async () => {
      if ((await $(testid('canvas')).getAttribute('aria-busy')) === 'true') {
        quiet = 0;
        previous = -1;
        return false;
      }

      const count = await $$(testid('note-card')).length;
      quiet = count === previous ? quiet + 1 : 0;
      previous = count;

      return quiet >= SETTLE_POLLS;
    },
    {
      timeout: 30_000,
      interval: SETTLE_INTERVAL,
      timeoutMsg: 'the canvas never settled',
    },
  );
}

/** The window's own box — not `getWindowSize`, which answers the OS frame with it. */
export async function viewportSize(): Promise<{ width: number; height: number }> {
  return browser.execute(() => ({ width: window.innerWidth, height: window.innerHeight }));
}

/** The cursor the WebView paints, which is what `all: unset` quietly takes away. */
export async function cursorOf(selector: string): Promise<string> {
  return browser.execute((sel: string) => {
    const element = document.querySelector(sel);
    if (!element) {
      throw new Error(`no element at ${sel}`);
    }
    return getComputedStyle(element).cursor;
  }, selector);
}

/**
 * Sets a field and checks it took, retrying if it did not. ⚠️ The editor's drafts are
 * `linkedSignal`s: a render landing between the click and the keystrokes rewrites
 * `[value]` from the note and wipes what was just typed.
 */
export async function setField(selector: string, text: string): Promise<void> {
  const field = $(selector);
  await field.waitForExist({ timeout: 10_000 });

  for (let attempt = 0; attempt < 3; attempt += 1) {
    await field.click();
    await field.setValue(text);

    if ((await field.getValue()) === text) return;
  }

  throw new Error(
    `${selector} would not hold ${JSON.stringify(text)} — it reads ${JSON.stringify(await field.getValue())}`,
  );
}

/**
 * ⚠️ The click crosses the bridge and comes back through a re-render; reading the list on
 * the next line reads it as it was before the click.
 */
export async function clickToAddRow(button: string, row: string) {
  const before = await $$(row).length;
  await $(button).click();

  await browser.waitUntil(async () => (await $$(row).length) > before, {
    timeout: 10_000,
    timeoutMsg: `no new ${row} after clicking ${button}`,
  });

  const rows = await $$(row).getElements();
  const last = rows.at(-1);
  if (!last) {
    throw new Error(`no ${row} to fill`);
  }

  return last;
}

/**
 * Waits for a read to answer something, and hands that answer back so the assertion reads
 * what was waited for.
 *
 * ⚠️ A condition rather than a duration. `pause` before an `expect` is a guess at a round
 * trip on a runner already sharing a CPU with a WebView: one scenario went red on Windows
 * and green on a re-run of the very same commit (#190).
 *
 * ⚠️ A **negative** assertion is the one case a duration is the right tool — nothing
 * arriving is not a condition anything can wait on. Those keep their `pause`, with a line
 * saying so.
 */
export async function eventually<T>(
  read: () => Promise<T>,
  matches: (value: T) => boolean,
  what: string,
): Promise<T> {
  let seen: T | undefined;

  await browser.waitUntil(
    async () => {
      seen = await read();
      return matches(seen);
    },
    { timeout: 10_000, timeoutMsg: `${what} — last saw ${JSON.stringify(seen)}` },
  );

  return read();
}

/** What `confirmTwice` needs of a button, so a chainable and an element both fit. */
interface Confirmable {
  click(): Promise<void>;
  getAttribute(name: string): Promise<string | null>;
}

/**
 * ⚠️ Destructive controls confirm on a second click. Clicking twice in a row bets the
 * first click's render has landed — and that render moves the button out from under it.
 */
export async function confirmTwice(button: Confirmable): Promise<void> {
  await button.click();

  await browser.waitUntil(async () => ((await button.getAttribute('class')) ?? '').includes('confirming'), {
    timeout: 10_000,
    timeoutMsg: 'the control never asked for confirmation',
  });

  await button.click();
}

/** What to take off each match: its text, an input's value, or `@some-attribute`. */
type Extract = 'text' | 'value' | `@${string}`;

/**
 * Reads one thing off every match, in a single call. ⚠️ A walk fetching one element per
 * round trip leaves a window in which the page re-renders, and the list that comes back
 * mixes two states.
 */
export async function readEach(selector: string, extract: Extract, child?: string): Promise<string[]> {
  return browser.execute(
    (parent: string, how: string, kid: string | null) =>
      [...document.querySelectorAll(parent)].map((element) => {
        const target = kid ? element.querySelector(kid) : element;
        if (!target) return '';
        if (how === 'text') return (target.textContent ?? '').trim();
        if (how === 'value') return (target as HTMLInputElement).value ?? '';
        return target.getAttribute(how.slice(1)) ?? '';
      }),
    selector,
    extract,
    child ?? null,
  );
}

/**
 * ⚠️ Writes here are not optimistic, so `aria-pressed` read on the line after the click
 * is still the state before it.
 */
export async function toggleAndWait(selector: string, attribute = 'aria-pressed'): Promise<void> {
  const button = $(selector);
  const before = await button.getAttribute(attribute);

  await button.click();

  await browser.waitUntil(async () => (await button.getAttribute(attribute)) !== before, {
    timeout: 10_000,
    timeoutMsg: `${selector} never reported a new ${attribute}`,
  });
}

export type Modifier = 'Control' | 'Alt' | 'Shift' | 'Meta';

/** `KeyboardEvent.code`: a letter is its physical key, a digit its own. */
function codeFor(key: string): string {
  if (/^[a-z]$/i.test(key)) {
    return `Key${key.toUpperCase()}`;
  }
  if (/^[0-9]$/.test(key)) {
    return `Digit${key}`;
  }
  return key;
}

/**
 * ⚠️ Not `browser.keys`: the embedded WebDriver server answers
 * `POST /session/:id/actions` with a 200 and dispatches nothing. A synthetic
 * `KeyboardEvent` reaches the same handlers, which all listen in the DOM.
 *
 * `code` is filled as carefully as `key`: the shortcut field reads the physical key, so
 * an event carrying only `key` records the wrong accelerator and passes for the wrong
 * reason. The native global shortcuts are out of reach for any WebDriver — see
 * `emitGlobalAction`.
 */
export async function press(key: string, modifiers: Modifier[] = []): Promise<void> {
  await browser.execute(
    (k: string, code: string, mods: string[]) => {
      const target: Element = document.activeElement ?? document.body;
      const init: KeyboardEventInit = {
        key: k,
        code,
        bubbles: true,
        cancelable: true,
        ctrlKey: mods.includes('Control'),
        altKey: mods.includes('Alt'),
        shiftKey: mods.includes('Shift'),
        metaKey: mods.includes('Meta'),
      };
      target.dispatchEvent(new KeyboardEvent('keydown', init));
      target.dispatchEvent(new KeyboardEvent('keyup', init));
    },
    key,
    codeFor(key),
    modifiers,
  );
}

/**
 * ⚠️ Neither `<select>` nor `<input type="date">` can be driven the obvious way:
 * `selectByAttribute` moves the selection without the `change` the components listen to,
 * and a date input accepts keystrokes in the display format, which follows the WebView's
 * locale. This skips the browser's own parsing; every handler downstream still runs.
 */
/**
 * Picks one entry of an `app-choice-menu`, which replaced every `<select>` but the date.
 *
 * ⚠️ Two clicks, not an assignment: these are menus, so there is no value to set — which is
 * the whole point of #179. A screen reader used to be told the selection bar held a combobox
 * whose current value was "Ranger dans".
 */
export async function pickChoice(kind: string, optionId: string | null): Promise<void> {
  await $(testid(`choice-${kind}`)).click();
  await $(testid(`choice-panel-${kind}`)).waitForExist({ timeout: 5_000 });
  const entry =
    optionId === null
      ? $(`${testid('choice-panel-' + kind)} ${testid('choice-none')}`)
      : $(`${testid('choice-panel-' + kind)} [data-option-id="${optionId}"]`);
  await entry.click();
}

/** Which entry a choice menu names on its trigger. */
export function choiceLabel(kind: string): Promise<string> {
  return $(testid(`choice-${kind}`)).getText();
}

/** Picks one of the few options a segmented control shows all at once. */
export async function pickSegment(label: string, segmentId: string): Promise<void> {
  await $(`[role="radiogroup"][aria-label="${label}"] [data-segment-id="${segmentId}"]`).click();
}

export function checkedSegment(label: string): Promise<string | null> {
  return $(`[role="radiogroup"][aria-label="${label}"] [aria-checked="true"]`).getAttribute(
    'data-segment-id',
  );
}

export async function setNativeValue(selector: string, value: string): Promise<void> {
  await $(selector).waitForExist({ timeout: 10_000 });
  await browser.execute(
    (sel: string, next: string) => {
      const field = document.querySelector(sel) as HTMLInputElement | HTMLSelectElement | null;
      if (!field) {
        throw new Error(`no field at ${sel}`);
      }
      field.value = next;
      field.dispatchEvent(new Event('change', { bubbles: true }));
    },
    selector,
    value,
  );
}

/**
 * ⚠️ Enter inside a text input submits through implicit submission, which the browser
 * reserves for real user input. `requestSubmit()` fires the event the form listens to.
 */
export async function submitFormOf(selector: string): Promise<void> {
  await browser.execute((sel: string) => {
    const field = document.querySelector(sel) as HTMLInputElement | null;
    field?.form?.requestSubmit();
  }, selector);
}

/**
 * ⚠️ Title, body and source commit on blur, and a synthetic key event does not move
 * focus — so the blur is asked for directly.
 */
export async function blur(): Promise<void> {
  await browser.execute(() => {
    (document.activeElement as HTMLElement | null)?.blur();
  });
}

/**
 * ⚠️ A note written straight through the bridge is invisible to a canvas that has not
 * been told to re-query: `NotesRevision` is a front-end signal, and the back end does
 * not push.
 */
/** ⚠️ The canvas keyboard ignores a keystroke aimed at a text field, by design. */
export async function blurField(): Promise<void> {
  await browser.execute(() => (document.activeElement as HTMLElement | null)?.blur());
}

/** Where a box is on screen, rounded — enough to say whether it moved. */
export async function boxOf(selector: string): Promise<{ left: number; top: number }> {
  return browser.execute((sel: string) => {
    const element = document.querySelector(sel);
    if (!element) throw new Error('no element at ' + sel);

    const box = element.getBoundingClientRect();
    return { left: Math.round(box.left), top: Math.round(box.top) };
  }, selector);
}

/**
 * Whether a field's placeholder fits at the narrowest its box can get — its `min-width`,
 * which is what a wrapping toolbar leaves it most of the time. A runner with a wide window
 * would otherwise measure a width nobody has.
 *
 * ⚠️ Measured by putting the placeholder in as a value and reading the scroll width: there
 * is no other way to ask a browser how wide a placeholder renders. Everything touched is
 * put back, and no event is dispatched, so the component never hears about it.
 */
export async function placeholderFitsAtItsFloor(field: string, box: string): Promise<boolean> {
  return browser.execute(
    (fieldSelector: string, boxSelector: string) => {
      const input = document.querySelector(fieldSelector) as HTMLInputElement | null;
      const around = document.querySelector(boxSelector) as HTMLElement | null;
      if (!input || !around) throw new Error('nothing to measure');

      const floor = getComputedStyle(around).minWidth;
      const held = { value: input.value, width: around.style.width, max: around.style.maxWidth };

      around.style.width = floor;
      around.style.maxWidth = floor;
      input.value = input.placeholder;
      const fits = input.scrollWidth <= input.clientWidth;

      input.value = held.value;
      around.style.width = held.width;
      around.style.maxWidth = held.max;
      return fits;
    },
    field,
    box,
  );
}

/** How far a box stops short of the bottom of the window — negative when it runs past. */
export async function bottomGapOf(selector: string): Promise<number> {
  return browser.execute((sel: string) => {
    const element = document.querySelector(sel);
    if (!element) throw new Error('no element at ' + sel);
    return Math.round(window.innerHeight - element.getBoundingClientRect().bottom);
  }, selector);
}

/**
 * Whether what is drawn at the middle of a box is that box, rather than something over
 * it. ⚠️ Asked of the painting and not of a `z-index`: stacking contexts make the number
 * on its own say nothing.
 */
export async function isInFront(selector: string): Promise<boolean> {
  return browser.execute((sel: string) => {
    const element = document.querySelector(sel);
    if (!element) throw new Error('no element at ' + sel);

    const box = element.getBoundingClientRect();
    const painted = document.elementFromPoint(box.left + box.width / 2, box.top + box.height / 2);
    return painted !== null && element.contains(painted);
  }, selector);
}

/** Which control really holds the keyboard, by its test id — `null` for anything else. */
export async function activeTestId(): Promise<string | null> {
  return browser.execute(() => document.activeElement?.getAttribute('data-testid') ?? null);
}

export async function reloadCanvas(): Promise<void> {
  await browser.refresh();
  await waitForCanvas();
}

/**
 * ⚠️ Not a restart of the application, and it cannot be one: under the `embedded`
 * provider the WebDriver server lives inside the process, which has to stay up. The Rust
 * side, its SQLite connection and the store plugin's map all survive.
 *
 * What it buys is a front end built from nothing — every store reconstructed from what
 * the commands answer. ⚠️ Never use it to prove something reached the disk;
 * `15-preferences-on-disk.e2e.ts` reads the file from Node for that.
 */
export async function reopenSession(): Promise<void> {
  await browser.reloadSession();
  await waitForCanvas();
}

/**
 * The OS keystroke that normally sends this cannot be typed through a WebView, so the
 * event is emitted instead. What is exercised is everything downstream of it.
 */
export async function emitGlobalAction(action: GlobalAction): Promise<void> {
  await browser.executeAsync((name: string, done: (value: unknown) => void) => {
    const tauri = (window as unknown as Record<string, any>)['__TAURI__'];
    tauri.event
      .emit('devnotes:action', name)
      .then(() => done(null))
      .catch(() => done(null));
  }, action);
}

/**
 * What the application put on the system clipboard, or `null` when this machine will not
 * let anyone read it. ⚠️ On Windows the clipboard is a single global lock a clipboard
 * manager can hold indefinitely, and a headless Linux runner may have no selection owner
 * — a property of the runner, so a caller skips rather than fails and asserts on
 * `DisplayNote.copyText` instead.
 */
export async function clipboardText(): Promise<string | null> {
  for (let attempt = 0; attempt < 10; attempt += 1) {
    const answer = (await browser.executeAsync((done: (value: unknown) => void) => {
      const tauri = (window as unknown as Record<string, any>)['__TAURI__'];
      tauri.core
        .invoke('plugin:clipboard-manager|read_text')
        .then((value: unknown) => done({ ok: typeof value === 'string' ? value : '' }))
        .catch((error: unknown) => done({ err: String(error) }));
    })) as { ok?: string; err?: string };

    if (answer.ok !== undefined) {
      return answer.ok;
    }
    await browser.pause(150);
  }
  return null;
}

/**
 * ⚠️ There is no way to stand in front of the OS file picker.
 * `window.__TAURI_INTERNALS__.invoke` is defined `writable: false, configurable: false`,
 * so neither an assignment nor `browser.tauri.mock()` can wrap it — a picker opened by a
 * click blocks the whole application until a human clicks it.
 *
 * So import, export and attaching are exercised through their commands, which take a
 * path. The controls that open a picker are asserted on, never clicked.
 */
