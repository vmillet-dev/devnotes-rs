import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { UsedVariable } from '@core/model/http-environment.model';
import { provideAppTesting } from '@testing/testing.providers';
import { VariableChipComponent } from './variable-chip.component';

const TOKEN: UsedVariable = {
  name: 'token',
  resolved: {
    name: 'token',
    value: 't0k3n',
    kind: 'secret',
    scope: 'environment',
    from: 'Staging',
    shadows: ['globals'],
  },
};

describe('VariableChipComponent', () => {
  let fixture: ComponentFixture<VariableChipComponent>;
  let defined: string[];

  async function render(variable: UsedVariable, defineIn: string | null = 'Staging') {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({ imports: [VariableChipComponent], providers: [provideAppTesting()] });
    fixture = TestBed.createComponent(VariableChipComponent);
    fixture.componentRef.setInput('variable', variable);
    fixture.componentRef.setInput('defineIn', defineIn);
    defined = [];
    fixture.componentInstance.defined.subscribe((name) => defined.push(name));
    fixture.autoDetectChanges();
    await fixture.whenStable();
  }

  const el = <E extends HTMLElement = HTMLElement>(selector: string): E =>
    fixture.nativeElement.querySelector(selector) as E;

  beforeEach(async () => {
    await render(TOKEN);
  });

  it('wears the scope it resolves from, and says it with what it hides', () => {
    expect(el('[data-testid="http-used-variable"]').dataset['scope']).toBe('environment');
    expect(el('button').textContent?.trim()).toBe('{{token}}');
    const tip = el('[data-testid="http-used-variable-tip"]').textContent ?? '';
    expect(tip).toContain('Staging');
    expect(tip).toContain('globales');
  });

  it('masks a secret until asked, and masks it again once left', async () => {
    const value = () => el('[data-testid="http-used-variable-value"]').textContent?.trim();
    expect(value()).toBe('••••••••');

    el('[data-testid="http-used-variable-reveal"]').click();
    await fixture.whenStable();
    expect(value()).toBe('t0k3n');

    el('[data-testid="http-used-variable"]').dispatchEvent(new PointerEvent('pointerleave'));
    await fixture.whenStable();
    expect(value()).toBe('••••••••');
  });

  it('shows a plain value as it is sent', async () => {
    await render({
      name: 'baseUrl',
      resolved: { ...TOKEN.resolved!, name: 'baseUrl', value: 'https://api', kind: 'text', shadows: [] },
    });

    expect(el('[data-testid="http-used-variable-value"]').textContent?.trim()).toBe('https://api');
    expect(el('[data-testid="http-used-variable-reveal"]')).toBeNull();
  });

  it('says what nothing sets, and offers to set it where the request is sent from', async () => {
    await render({ name: 'tenantId', resolved: null }, 'Staging');

    expect(el('[data-testid="http-used-variable"]').dataset['scope']).toBe('none');
    expect(el('[data-testid="http-used-variable-tip"]').textContent).toContain('introuvable');
    const define = el('[data-testid="http-used-variable-define"]');
    expect(define.textContent?.trim()).toBe('Définir dans Staging');
    define.click();
    expect(defined).toEqual(['tenantId']);

    await render({ name: 'tenantId', resolved: null }, null);
    expect(el('[data-testid="http-used-variable-define"]').textContent?.trim()).toBe(
      'Définir dans les globales',
    );
  });
});
