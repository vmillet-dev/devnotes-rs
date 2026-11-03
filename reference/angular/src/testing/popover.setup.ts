/**
 * jsdom has no `showPopover`, and since 30.1.2 it refuses the focus to what is not rendered —
 * a closed popover included, hidden by its own default stylesheet. Opening one here marks it,
 * and a rule shows what is marked, so a spec sees a panel as a browser would: focusable once
 * shown, not before. `!important` because jsdom ranks its stylesheet by specificity alone.
 */
const OPEN = 'data-test-popover-open';

if (typeof HTMLElement !== 'undefined' && typeof HTMLElement.prototype.showPopover !== 'function') {
  HTMLElement.prototype.showPopover = function (this: HTMLElement) {
    this.setAttribute(OPEN, '');
  };
  HTMLElement.prototype.hidePopover = function (this: HTMLElement) {
    this.removeAttribute(OPEN);
  };

  const style = document.createElement('style');
  style.textContent = `[popover][${OPEN}] { display: block !important; }`;
  document.head.append(style);
}
