import { ComponentFixture, TestBed } from '@angular/core/testing';
import { By } from '@angular/platform-browser';
import { beforeEach, describe, expect, it } from 'vitest';
import { LanguageTag } from '@core/model/language.model';
import { loadGrammar } from '@core/services/syntax/grammars';
import { CodeViewerComponent } from './code-viewer.component';

describe('CodeViewerComponent', () => {
  let fixture: ComponentFixture<CodeViewerComponent>;

  /** Scope of the token covering `text`, or undefined when it is not highlighted. */
  function scopeOf(text: string): string | undefined {
    const token = fixture.debugElement
      .queryAll(By.css('.line-content span'))
      .find((span) => span.nativeElement.textContent === text);

    return token && Object.keys(token.classes).find((cls) => cls.startsWith('tok-'));
  }

  /** Its grammar arrives after the first drawing, which is plain text. */
  async function show(content: string, language: LanguageTag): Promise<void> {
    fixture.componentRef.setInput('content', content);
    fixture.componentRef.setInput('language', language);
    fixture.detectChanges();
    await loadGrammar(language);
    await fixture.whenStable();
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
    await show('{"name": "value", "count": 2}', 'json');

    expect(scopeOf('"name"')).toBe('tok-property');
    expect(scopeOf('"value"')).toBe('tok-string');
    expect(scopeOf('2')).toBe('tok-literal');
  });

  it('draws plain text until the grammar has arrived, then colours it', async () => {
    fixture.componentRef.setInput('content', 'local x = 1');
    fixture.componentRef.setInput('language', 'lua');
    fixture.detectChanges();

    expect(fixture.debugElement.queryAll(By.css('.line-content span'))).toHaveLength(0);

    await loadGrammar('lua');
    await fixture.whenStable();

    expect(scopeOf('local')).toBe('tok-keyword');
  });

  it.each<[LanguageTag, string, string, string]>([
    ['js', 'const total = 1;', 'const', 'tok-keyword'],
    ['ts', 'interface Shape {}', 'interface', 'tok-keyword'],
    ['py', 'def run():', 'def', 'tok-keyword'],
    ['sql', 'SELECT id FROM notes', 'SELECT', 'tok-keyword'],
    ['yml', 'key: value', 'key', 'tok-property'],
    ['sh', 'echo "hi"', '"hi"', 'tok-string'],
    ['css', '.card { color: red; }', 'color', 'tok-property'],
    ['tf', 'resource "x" "y" {}', 'resource', 'tok-keyword'],
    ['dockerfile', 'FROM node:20', 'FROM', 'tok-keyword'],
  ])('colours %s content', async (language, content, text, scope) => {
    await show(content, language);

    expect(scopeOf(text)).toBe(scope);
  });

  it('keeps a multi-line comment coloured across every one of its lines', async () => {
    await show('/* one\n   two\n   three */\nconst after = 1;', 'js');

    const comments = fixture.debugElement.queryAll(By.css('.line-content .tok-comment'));
    expect(comments.map((node) => node.nativeElement.textContent)).toEqual([
      '/* one',
      '   two',
      '   three */',
    ]);
    expect(scopeOf('const')).toBe('tok-keyword');
  });

  it('does not fail on content its grammar cannot parse', async () => {
    await show('{"unclosed": ', 'json');

    expect(fixture.debugElement.query(By.css('.line-content')).nativeElement.textContent).toBe(
      '{"unclosed": ',
    );
  });
});
