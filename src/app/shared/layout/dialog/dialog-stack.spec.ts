import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { DialogStack } from './dialog-stack';
import { dialogRung } from './dialog.model';

describe('DialogStack', () => {
  let stack: DialogStack;

  const editor = { name: 'editor' };
  const palette = { name: 'palette' };
  const fields = { name: 'fields' };

  beforeEach(() => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({});
    stack = TestBed.inject(DialogStack);
  });

  it('says nothing is open before anything opens', () => {
    expect(stack.hasOpenDialog()).toBe(false);
    expect(stack.isFront(editor)).toBe(false);
  });

  /** What tells a page its keyboard is taken. */
  it('reports an open dialog for as long as one is open', () => {
    stack.push(editor, dialogRung('editor'));
    expect(stack.hasOpenDialog()).toBe(true);

    stack.remove(editor);
    expect(stack.hasOpenDialog()).toBe(false);
  });

  it('gives the keystroke to the only dialog there is', () => {
    stack.push(editor, dialogRung('editor'));

    expect(stack.isFront(editor)).toBe(true);
    expect(stack.isFront(palette)).toBe(false);
  });

  /**
   * The reason the stack orders by rung and not by arrival: the fields form is created
   * *after* the palette in the DOM but is drawn in front of it, and Escape has to follow
   * what is on screen rather than what was built last.
   */
  it('follows the rung, not the order things were opened', () => {
    stack.push(fields, dialogRung('fields'));
    stack.push(palette, dialogRung('palette'));

    expect(dialogRung('fields')).toBeGreaterThan(dialogRung('palette'));
    expect(stack.isFront(fields)).toBe(true);
    expect(stack.isFront(palette)).toBe(false);
  });

  it('hands the front back to what is left when the top one closes', () => {
    stack.push(palette, dialogRung('palette'));
    stack.push(fields, dialogRung('fields'));

    stack.remove(fields);

    expect(stack.isFront(palette)).toBe(true);
  });

  /** Two dialogs on one rung: the later one is in front, which is the DOM order. */
  it('breaks a tie in favour of the one opened last', () => {
    const first = { name: 'first' };
    const second = { name: 'second' };
    stack.push(first, dialogRung('app'));
    stack.push(second, dialogRung('app'));

    expect(stack.isFront(second)).toBe(true);
    expect(stack.isFront(first)).toBe(false);
  });

  it('forgets a dialog that was never there without complaining', () => {
    stack.push(editor, dialogRung('editor'));

    expect(() => stack.remove(palette)).not.toThrow();
    expect(stack.isFront(editor)).toBe(true);
  });

  it('tracks owners by identity, so two dialogs of one kind are two entries', () => {
    const left = { layer: 'app' };
    const right = { layer: 'app' };
    stack.push(left, dialogRung('app'));
    stack.push(right, dialogRung('app'));

    stack.remove(right);

    expect(stack.hasOpenDialog()).toBe(true);
    expect(stack.isFront(left)).toBe(true);
  });
});
