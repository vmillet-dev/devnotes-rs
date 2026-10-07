import { ComponentFixture, TestBed } from '@angular/core/testing';
import { By } from '@angular/platform-browser';
import { beforeEach, describe, expect, it } from 'vitest';
import { CodeViewerComponent } from './code-viewer.component';

describe('CodeViewerComponent', () => {
  let fixture: ComponentFixture<CodeViewerComponent>;

  /** Scope of the token covering `text`, or undefined when it is not highlighted. */
  function scopeOf(text: string): string | undefined {
    const token = fixture.debugElement
      .queryAll(By.css('.line-content span'))
      .find((span) => span.nativeElement.textContent === text);

    return token && Object.keys(token.classes).find((cls) => cls.startsWith('hljs-'));
  }

  beforeEach(() => {
    TestBed.configureTestingModule({ imports: [CodeViewerComponent] });
    fixture = TestBed.createComponent(CodeViewerComponent);
    fixture.componentRef.setInput('content', '');
    fixture.autoDetectChanges();
  });

  it('renders one line number per content line', async () => {
    fixture.componentRef.setInput('content', 'line one\nline two\nline three');
    await fixture.whenStable();

    const lineNumbers = fixture.debugElement.queryAll(By.css('.line-no'));
    expect(lineNumbers.map((el) => el.nativeElement.textContent)).toEqual(['1', '2', '3']);
  });

  it('drops the gutter when line numbers are turned off', async () => {
    fixture.componentRef.setInput('content', 'one\ntwo');
    fixture.componentRef.setInput('showLineNumbers', false);
    await fixture.whenStable();

    expect(fixture.debugElement.queryAll(By.css('.line-no'))).toHaveLength(0);
    expect(fixture.debugElement.queryAll(By.css('.line-content'))).toHaveLength(2);
  });

  it('leaves plain text untouched, since txt has nothing to colour', async () => {
    fixture.componentRef.setInput('content', 'const x = 1;');
    await fixture.whenStable();

    const line = fixture.debugElement.query(By.css('.line-content'));
    expect(line.nativeElement.textContent).toBe('const x = 1;');
    expect(line.queryAll(By.css('span'))).toHaveLength(0);
  });

  it('escapes markup rather than rendering it', async () => {
    fixture.componentRef.setInput('content', '<img src=x onerror="boom">');
    await fixture.whenStable();

    const line = fixture.debugElement.query(By.css('.line-content'));
    expect(line.nativeElement.textContent).toBe('<img src=x onerror="boom">');
    expect(line.queryAll(By.css('img'))).toHaveLength(0);
  });

  it('colours JSON keys, strings and numbers', async () => {
    fixture.componentRef.setInput('content', '{"name": "value", "count": 2}');
    fixture.componentRef.setInput('language', 'json');
    await fixture.whenStable();

    expect(scopeOf('"name"')).toBe('hljs-attr');
    expect(scopeOf('"value"')).toBe('hljs-string');
    expect(scopeOf('2')).toBe('hljs-number');
  });

  it.each([
    ['js', 'const total = 1;', 'const', 'hljs-keyword'],
    ['ts', 'interface Shape {}', 'interface', 'hljs-keyword'],
    ['py', 'def run():', 'def', 'hljs-keyword'],
    ['sql', 'SELECT id FROM notes', 'SELECT', 'hljs-keyword'],
    ['yml', 'key: value', 'key:', 'hljs-attr'],
    ['sh', 'echo "hi"', '"hi"', 'hljs-string'],
    ['css', '.card { color: red; }', 'color', 'hljs-attribute'],
  ])('colours %s content', async (language, content, text, scope) => {
    fixture.componentRef.setInput('content', content);
    fixture.componentRef.setInput('language', language);
    await fixture.whenStable();

    expect(scopeOf(text)).toBe(scope);
  });

  it('keeps a multi-line comment coloured across every one of its lines', async () => {
    fixture.componentRef.setInput('content', '/* one\n   two\n   three */\nconst after = 1;');
    fixture.componentRef.setInput('language', 'js');
    await fixture.whenStable();

    const comments = fixture.debugElement.queryAll(By.css('.line-content .hljs-comment'));
    expect(comments.map((node) => node.nativeElement.textContent)).toEqual([
      '/* one',
      '   two',
      '   three */',
    ]);
    expect(scopeOf('const')).toBe('hljs-keyword');
  });

  it('does not fail on content its grammar cannot parse', async () => {
    fixture.componentRef.setInput('content', '{"unclosed": ');
    fixture.componentRef.setInput('language', 'json');
    await fixture.whenStable();

    expect(fixture.debugElement.query(By.css('.line-content')).nativeElement.textContent).toBe(
      '{"unclosed": ',
    );
  });
});
