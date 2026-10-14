/**
 * ⚠️ What one WebDriver command may take. On WebKitGTK, a script the embedded server sends while
 * the page unloads is never answered, and the server holds it for the session's script timeout:
 * 30 s by default, which ended the waits it fell in although the next page had long been drawn.
 * Shorter, the lost read fails, and the wait's next one goes to the page that replaced it.
 * Set on every session the run opens.
 */
export const SCRIPT_TIMEOUT = 5_000;

/** A key derived at its real cost, and one re-wrap per retained backup, on a shared runner. */
export const SLOW_SCRIPT_TIMEOUT = 120_000;
