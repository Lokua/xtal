export default {
  semi: false,
  singleQuote: true,
  printWidth: 80,
  proseWrap: 'always',
  trailingComma: 'all',
  overrides: [
    {
      files: ['*.yml', '*.yaml', '*.md'],
      options: {
        tabWidth: 2,
        useTabs: false,
      },
    },
  ],
}
