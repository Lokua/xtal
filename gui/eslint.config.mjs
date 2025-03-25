import lokuaConfig from 'eslint-config-lokua'
import react from 'eslint-plugin-react'
import globals from 'globals'

export default [
  ...lokuaConfig,
  {
    files: ['**/*.js', '**/*.jsx', '**/*.mjs'],
    languageOptions: {
      parserOptions: {
        sourceType: 'module',
        ecmaFeatures: {
          jsx: true,
        },
      },
      globals: {
        ...globals.browser,
      },
    },
    plugins: {
      react,
    },
    settings: {
      react: {
        version: 'detect',
      },
    },
    rules: {
      'block-scoped-var': 'error',
      'object-shorthand': 'error',
      'prefer-const': 'error',
      'no-undef': 'error',
      'no-unused-vars': [
        'error',
        {
          varsIgnorePattern: (() => ['^_'].join('|'))(),
          args: 'after-used',
          argsIgnorePattern: '^_',
        },
      ],
      'no-use-before-define': [
        'error',
        {
          functions: false,
          classes: false,
        },
      ],
      'react/jsx-no-undef': 'error',
      'react/jsx-uses-vars': 'error',
      'react/jsx-uses-react': 'error',
    },
  },
]
