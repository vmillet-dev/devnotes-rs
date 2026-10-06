import { describe, expect, it, vi } from 'vitest';
import { ClassRights, Mode, PermissionsAnswer } from '@core/model/tool-answers.model';
import { FakeToolsRepository } from '@testing/fake-tools-repository';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { PermissionsToolComponent } from './permissions-tool.component';

const rights = (
  row: ClassRights['class'],
  read: boolean,
  write: boolean,
  execute: boolean,
  special = false,
) => ({
  class: row,
  read,
  write,
  execute,
  special,
});

const MODE_755: Mode = {
  bits: 0o755,
  octal: '755',
  symbolic: 'rwxr-xr-x',
  chmodSymbolic: 'u=rwx,g=rx,o=rx',
  classes: [
    rights('owner', true, true, true),
    rights('group', true, false, true),
    rights('others', true, false, true),
  ],
};

const UMASK: PermissionsAnswer['umask'] = {
  kind: 'read',
  file: { ...MODE_755, bits: 0o644, octal: '644', symbolic: 'rw-r--r--' },
  directory: MODE_755,
};

const ANSWER: PermissionsAnswer = {
  mode: { kind: 'read', mode: MODE_755, fileType: null, warnings: [] },
  umask: UMASK,
};

describe('PermissionsToolComponent', () => {
  const answering =
    (answer: PermissionsAnswer) =>
    (tools: FakeToolsRepository): void => {
      tools.permissions = answer;
    };

  const asked = (harness: ToolHarness<PermissionsToolComponent>) =>
    harness.tools.requestsOf('describe_permissions');
  const field = (harness: ToolHarness<PermissionsToolComponent>, testid: string) =>
    harness.element<HTMLInputElement>(`[data-testid="${testid}"]`);

  it('reads the umask alone at first, and draws the boxes unticked', async () => {
    const harness = await renderTool(
      PermissionsToolComponent,
      answering({ mode: { kind: 'empty' }, umask: UMASK }),
    );
    await vi.waitFor(() => expect(asked(harness)).toHaveLength(1));
    await harness.settle();

    expect(asked(harness)[0]).toEqual({ mode: '', umask: '022' });
    expect(harness.all('[data-testid="permissions-grid"] input:checked')).toHaveLength(0);
    expect(
      harness.element('[data-testid="permissions-new-file"] [data-testid="output-value"]').textContent,
    ).toBe('644 · rw-r--r--');
    expect(
      harness.element('[data-testid="permissions-new-directory"] [data-testid="output-value"]').textContent,
    ).toBe('755 · rwxr-xr-x');
    expect(harness.tool.result()).toBeNull();
  });

  it('writes an octal mode in letters, in the boxes, as commands and in words', async () => {
    const harness = await renderTool(PermissionsToolComponent, answering(ANSWER));

    await harness.type('permissions-octal', '755', 'describe_permissions');

    expect(field(harness, 'permissions-symbolic').value).toBe('rwxr-xr-x');
    expect(field(harness, 'permissions-owner-write').checked).toBe(true);
    expect(field(harness, 'permissions-group-write').checked).toBe(false);
    expect(harness.element('[data-command="octal"] [data-testid="output-value"]').textContent).toBe(
      'chmod 755 fichier',
    );
    expect(harness.element('[data-command="symbolic"] [data-testid="output-value"]').textContent).toBe(
      'chmod u=rwx,g=rx,o=rx fichier',
    );
    expect(harness.element('[data-testid="permissions-words"] [data-class="group"]').textContent).toContain(
      'lire, exécuter',
    );
  });

  it('reads letters typed, and writes them back as octal', async () => {
    const harness = await renderTool(
      PermissionsToolComponent,
      answering({ ...ANSWER, mode: { kind: 'read', mode: MODE_755, fileType: 'directory', warnings: [] } }),
    );

    await harness.type('permissions-symbolic', 'drwxr-xr-x', 'describe_permissions');

    expect(asked(harness).at(-1)).toEqual({ mode: 'drwxr-xr-x', umask: '022' });
    expect(field(harness, 'permissions-octal').value).toBe('755');
    expect(field(harness, 'permissions-symbolic').value).toBe('drwxr-xr-x');
    expect(harness.element('[data-testid="permissions-type"]').textContent).toContain('dossier');
  });

  it('flips one bit per box, special bits included', async () => {
    const harness = await renderTool(PermissionsToolComponent, answering(ANSWER));
    await harness.type('permissions-octal', '755', 'describe_permissions');

    field(harness, 'permissions-group-write').dispatchEvent(new Event('change'));
    await vi.waitFor(() => expect(asked(harness).at(-1)).toMatchObject({ mode: '775' }));
    harness.element<HTMLButtonElement>('[data-testid="permissions-owner-special"]').click();
    await vi.waitFor(() => expect(asked(harness).at(-1)).toMatchObject({ mode: '4755' }));

    expect(field(harness, 'permissions-octal').value).toBe('4755');
  });

  it('ticks a box from nothing', async () => {
    const harness = await renderTool(
      PermissionsToolComponent,
      answering({ mode: { kind: 'empty' }, umask: UMASK }),
    );
    await vi.waitFor(() => expect(asked(harness)).toHaveLength(1));

    field(harness, 'permissions-others-read').dispatchEvent(new Event('change'));

    await vi.waitFor(() => expect(asked(harness).at(-1)).toMatchObject({ mode: '004' }));
  });

  it('names the special bits set, in words', async () => {
    const special: Mode = {
      ...MODE_755,
      bits: 0o1777,
      octal: '1777',
      classes: [
        rights('owner', true, true, true),
        rights('group', true, true, true),
        rights('others', true, true, true, true),
      ],
    };
    const harness = await renderTool(
      PermissionsToolComponent,
      answering({ ...ANSWER, mode: { kind: 'read', mode: special, fileType: null, warnings: [] } }),
    );

    await harness.type('permissions-octal', '1777', 'describe_permissions');

    expect(harness.element('[data-testid="permissions-words"] [data-class="others"]').textContent).toContain(
      'sticky',
    );
    expect(harness.element('[data-testid="permissions-others-special"]').getAttribute('aria-pressed')).toBe(
      'true',
    );
    expect(harness.element('[data-testid="permissions-owner-special"]').getAttribute('aria-pressed')).toBe(
      'false',
    );
  });

  it('says why a mode is probably a mistake, in the words of the warning Rust names', async () => {
    const harness = await renderTool(PermissionsToolComponent, (tools) => {
      tools.permissions = {
        ...ANSWER,
        mode: {
          kind: 'read',
          mode: MODE_755,
          fileType: null,
          warnings: ['othersOverGroup', 'worldWritable'],
        },
      };
    });

    await harness.type('permissions-octal', '501', 'describe_permissions');

    expect(
      harness.all('[data-testid="permissions-warning"]').map((warning) => warning.dataset['warning']),
    ).toEqual(['othersOverGroup', 'worldWritable']);
    expect(harness.element('[data-testid="permissions-warning"]').textContent).toContain(
      'le groupe a moins de droits que tout le monde',
    );
  });

  it('says where a mode or a umask stops making sense', async () => {
    const harness = await renderTool(
      PermissionsToolComponent,
      answering({
        mode: { kind: 'refused', problem: 'notOctal', at: 3 },
        umask: { kind: 'refused', problem: 'unexpected', at: 2 },
      }),
    );

    await harness.type('permissions-octal', '758', 'describe_permissions');

    expect(harness.element('[data-testid="permissions-problem"]').textContent).toContain('Caractère 3');
    expect(harness.element('[data-testid="permissions-umask-problem"]').textContent).toContain('Caractère 2');
    expect(field(harness, 'permissions-symbolic').value).toBe('');
  });

  it('asks again as the umask is typed', async () => {
    const harness = await renderTool(PermissionsToolComponent, answering(ANSWER));

    await harness.type('permissions-umask', '077', 'describe_permissions');

    expect(asked(harness).at(-1)).toEqual({ mode: '', umask: '077' });
  });

  it('keeps both commands as a shell note', async () => {
    const harness = await renderTool(PermissionsToolComponent, answering(ANSWER));

    await harness.type('permissions-octal', '755', 'describe_permissions');

    expect(harness.tool.result()).toEqual({
      title: { key: 'tools.permissions.noteTitle', params: { octal: '755' } },
      kind: 'snippet',
      language: 'sh',
      content: 'chmod 755 fichier\nchmod u=rwx,g=rx,o=rx fichier\n# rwxr-xr-x',
    });
  });

  it('empties the mode and the umask on Vider', async () => {
    const harness = await renderTool(PermissionsToolComponent, answering(ANSWER));
    await harness.type('permissions-octal', '755', 'describe_permissions');

    harness.tool.clear();
    await harness.settle();

    expect(field(harness, 'permissions-octal').value).toBe('');
    expect(field(harness, 'permissions-umask').value).toBe('');
  });
});
