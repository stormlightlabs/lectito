// @ts-check
import { includeIgnoreFile } from '@eslint/compat';
import js from '@eslint/js';
import prettier from 'eslint-config-prettier';
import svelte from 'eslint-plugin-svelte';
import unicorn from 'eslint-plugin-unicorn';
import { defineConfig } from 'eslint/config';
import globals from 'globals';
import path from 'node:path';
import ts from 'typescript-eslint';
import svelteConfig from './svelte.config.js';

/** @typedef {import("eslint").Linter.Config} FlatConfig */
const unicornConfig = /** @type {FlatConfig} */ (/** @type {unknown} */ (unicorn.configs.recommended));

const gitignorePath = path.resolve(import.meta.dirname, '.gitignore');

export default defineConfig(
	includeIgnoreFile(gitignorePath),
	js.configs.recommended,
	ts.configs.recommended,
	svelte.configs.recommended,
	prettier,
	svelte.configs.prettier,
	unicornConfig,

	// typescript-eslint strongly recommend that you do not use the no-undef lint rule on TypeScript projects.
	// see: https://typescript-eslint.io/troubleshooting/faqs/eslint/#i-get-errors-from-the-no-undef-rule-about-global-variables-not-being-defined-even-though-there-are-no-typescript-errors
	{ languageOptions: { globals: { ...globals.browser, ...globals.node } }, rules: { 'no-undef': 'off' } },
	{
		files: ['**/*.svelte', '**/*.svelte.ts', '**/*.svelte.js'],
		languageOptions: {
			parserOptions: { projectService: true, extraFileExtensions: ['.svelte'], parser: ts.parser, svelteConfig }
		}
	},
	{
		rules: {
			'unicorn/catch-error-name': 'off',
			'unicorn/filename-case': 'off',
			'unicorn/no-negated-condition': 'off',
			'unicorn/no-null': 'off',
			'unicorn/prefer-query-selector': 'off',
			'unicorn/prefer-top-level-await': 'off',
			'unicorn/prevent-abbreviations': 'off',
			'unicorn/prefer-ternary': 'off',
			'unicorn/switch-case-braces': 'warn',
			'@typescript-eslint/no-unused-vars': ['warn', { argsIgnorePattern: '^_' }]
		}
	}
);
