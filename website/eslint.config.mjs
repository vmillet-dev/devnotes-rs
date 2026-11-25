// @ts-check
import eslint from '@eslint/js';
import tseslint from 'typescript-eslint';
import angular from 'angular-eslint';

export default tseslint.config(
  {
    ignores: ['dist/**', 'out-tsc/**', 'node_modules/**', '.angular/**', 'coverage/**'],
  },
  {
    files: ['**/*.ts'],
    extends: [
      eslint.configs.recommended,
      // Type-checked, not just syntactic: `no-floating-promises` and its kin.
      ...tseslint.configs.recommendedTypeChecked,
      ...tseslint.configs.stylistic,
      ...angular.configs.tsRecommended,
    ],
    languageOptions: {
      parserOptions: {
        projectService: true,
        tsconfigRootDir: import.meta.dirname,
      },
    },
    processor: angular.processInlineTemplates,
    rules: {
      '@angular-eslint/directive-selector': [
        'error',
        { type: 'attribute', prefix: 'app', style: 'camelCase' },
      ],
      '@angular-eslint/component-selector': [
        'error',
        { type: 'element', prefix: 'app', style: 'kebab-case' },
      ],

      // Zoneless: a component without OnPush is a silent performance bug.
      '@angular-eslint/prefer-on-push-component-change-detection': 'error',

      // Inputs and outputs go through the signal functions, not the decorators.
      '@angular-eslint/prefer-signals': 'error',
      '@angular-eslint/prefer-output-emitter-ref': 'error',

      // `_` prefixes a store's private writable signals, exposed read-only.
      '@typescript-eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_', varsIgnorePattern: '^_' }],
      '@typescript-eslint/consistent-type-definitions': ['error', 'interface'],

      // A `switch` over a union must stay exhaustive: a variant added has to break the build.
      '@typescript-eslint/switch-exhaustiveness-check': 'error',

      // A class's state in one place: fields, then the constructor, then methods. Field
      // initialisers run in declaration order, so a field that reads another stays below it.
      '@typescript-eslint/member-ordering': [
        'error',
        { default: ['signature', 'field', 'static-initialization', 'constructor', 'method'] },
      ],
      'no-duplicate-imports': ['error', { allowSeparateTypeImports: true }],
    },
  },
  {
    // The build scripts are neither Angular nor TypeScript: plain Node ESM, so the base rules
    // and the globals they reach for. Without this block a `.mjs` is linted with none.
    files: ['scripts/**/*.mjs'],
    extends: [eslint.configs.recommended],
    languageOptions: {
      globals: { console: 'readonly', process: 'readonly', URL: 'readonly', fetch: 'readonly' },
    },
  },
  {
    files: ['**/*.html'],
    extends: [...angular.configs.templateRecommended, ...angular.configs.templateAccessibility],
    rules: {
      // The site is entirely keyboard-navigable: these must not be warnings.
      '@angular-eslint/template/click-events-have-key-events': 'error',
      '@angular-eslint/template/interactive-supports-focus': 'error',
      '@angular-eslint/template/label-has-associated-control': 'error',
      '@angular-eslint/template/valid-aria': 'error',
      '@angular-eslint/template/role-has-required-aria': 'error',
    },
  },
  {
    // Specs may be less strict: internals, partial doubles.
    files: ['**/*.spec.ts', 'src/testing/**/*.ts'],
    rules: {
      '@typescript-eslint/no-non-null-assertion': 'off',
      '@typescript-eslint/no-explicit-any': 'off',
      '@typescript-eslint/no-unsafe-assignment': 'off',
      '@typescript-eslint/no-unsafe-member-access': 'off',
      '@typescript-eslint/no-unsafe-argument': 'off',
      '@typescript-eslint/no-unsafe-call': 'off',
      '@typescript-eslint/no-unsafe-return': 'off',

      // Assertions on queried DOM nodes are deliberately explicit.
      '@typescript-eslint/require-await': 'off',
      '@typescript-eslint/no-unnecessary-type-assertion': 'off',
      '@typescript-eslint/unbound-method': 'off',
    },
  },
);
