# Terminal demo

Activate the integration for your shell first. The following commands create a disposable demo repository. Run them in a scratch directory:

```sh
mkdir git-autocomplete-demo
cd git-autocomplete-demo
git init
mkdir -p src/auth tests/auth
printf 'export function validateLogin(value) { return value.length > 0; }\n' > src/auth/login.ts
printf 'console.assert(true);\n' > tests/auth/login.test.ts
git add .
git-autocomplete suggest --format json
```

Type `git commit -m ` and press Tab. An added exported `validateLogin` declaration produces a feature suggestion plus a conservative path summary. Use the native completion menu to select one. Press Ctrl+C to leave the demo without committing.

For a single-match demo, unstage the implementation, stage a README, and try completion again:

```sh
git rm --cached -r src tests
printf '# Demo\n' > README.md
git add README.md
git-autocomplete suggest
```

Tab after `git commit --message ` inserts `docs: add README` as one argument. Type your own nonempty message and press Tab to confirm it stays yours. Nothing commits until you press Enter on a Git command.

Record with your terminal's recorder or a tool you already use. Ghost text is outside this release's scope.
