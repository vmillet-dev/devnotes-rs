// Only options the Angular `unit-test` builder does not already set belong here: it owns
// the environment, the file include pattern and the Angular-aware transform pipeline.

import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    coverage: {
      provider: 'v8',
      reporter: ['text', 'html', 'lcov'],
      // No `include`: the builder instruments the files it bundles, and the paths it
      // reports do not match a source-relative glob.
      exclude: [
        '**/*.spec.ts',
        // Fixtures and doubles: measuring them would only inflate the numbers.
        'src/testing/**',
        // Bootstrap and wiring, exercised by running the app rather than by units.
        'src/main.ts',
        '**/app.config.ts',
        '**/app.routes.ts',
        // Thin `invoke()` adapters: nothing to assert without a live Tauri runtime.
        '**/*.repository.ts',
        '**/ipc.service.ts',
        // Its message glue: what it runs is `prettier-runner.ts`, driven by a spec with the real Prettier.
        '**/*.worker.ts',
      ],
      thresholds: {
        statements: 80,
        branches: 80,
        functions: 80,
        lines: 80,
      },
    },
  },
});
