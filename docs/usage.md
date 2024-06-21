# Usage

## tach mod
`tach` comes bundled with a command to set up and define your initial boundaries.

```bash
usage: tach mod [-h] [-d [DEPTH]] [-e file_or_path,...]

Configure module boundaries interactively

options:
  -h, --help            show this help message and exit
  -d [DEPTH], --depth [DEPTH]
                        The number of child directories to expand from the root
  -e file_or_path,..., --exclude file_or_path,...
                        Comma separated path list to exclude. tests/, ci/, etc.
```

Running `tach mod` will open an interactive editor in your terminal which allows you to mark your module boundaries.

You can navigate with the arrow keys, mark individual modules with `Enter`, and mark all siblings
as modules with `Ctrl + a`.

You can also mark your Python [source root](configuration.md#source-root) by pressing `s`.
This allows Tach to understand the module paths used in your project, and to correctly identify first-party imports.

When you are ready to save your modules, use `Ctrl + s` to save and exit. Otherwise, to exit without saving you can use `Ctrl + c`.

Any time you make changes with `tach mod`, it is recommended to run [`tach sync`](usage.md#tach-sync)
to automatically configure dependency rules.


## tach check
`tach` will flag any unwanted imports between modules. We recommend you run `tach check` like a linter or test runner, e.g. in pre-commit hooks, on-save hooks, and in CI pipelines.

```bash
usage: tach check [-h] [--exact] [-e file_or_path,...]

Check existing boundaries against your dependencies and module interfaces

options:
  -h, --help            show this help message and exit
  --exact               Raise errors if any dependency constraints are unused.
  -e file_or_path,..., --exclude file_or_path,...
                        Comma separated path list to exclude. tests/, ci/, etc.
```

An error will indicate:

- the file path in which the error was detected
- the tag associated with that file
- the tag associated with the attempted import

If `--exact` is provided, additional errors will be raised if a dependency exists in `tach.yml` that is not exercised by the code.

Example:
```bash
> tach check
❌ tach/check.py[L8]: Cannot import 'tach.filesystem'. Tag 'tach' cannot depend on 'tach.filesystem'. 
```

NOTE: If your terminal supports hyperlinks, you can click on the failing file path to go directly to the error.


## tach sync
`tach` can automatically sync your project configuration (`tach.yml`) with your project's actual dependencies.

```bash
usage: tach sync [-h] [--prune] [-e file_or_path,...]

Sync constraints with actual dependencies in your project.

options:
  -h, --help            show this help message and exit
  --prune               Prune all existing constraints and re-sync dependencies.
  -e file_or_path,..., --exclude file_or_path,...
                        Comma separated path list to exclude. tests/, ci/, etc.
```

When this command runs, `tach` will analyze the imports in your Python project.

Any undeclared dependencies or other dependency errors will be automatically resolved by
adding the corresponding dependencies to your `tach.yml` file.

If you supply `--prune`,
any dependency constraints in your `tach.yml` which are not necessary will also be removed.

## tach report
`tach` can generate a report showing all the dependencies and usages of a given file or directory in your project.

```bash
usage: tach report [-h] [-e file_or_path,...] path

Create a report of dependencies and usages of the given filepath or directory.

positional arguments:
  path                  The filepath or directory path used to generate the report.

options:
  -h, --help            show this help message and exit
  -e file_or_path,..., --exclude file_or_path,...
                        Comma separated path list to exclude. tests/, ci/, etc.
```

This will generate a textual report showing the file and line number of each relevant import.

## tach install
`tach` can be installed into your development workflow automatically as a pre-commit hook.


### With pre-commit framework
If you use the [pre-commit framework](https://github.com/pre-commit/pre-commit), you can add the following to your `.pre-commit-hooks.yaml`:

```yaml
repos:
-   repo: https://github.com/gauge-sh/tach-pre-commit
    rev: v0.6.0  # change this to the latest tag!
    hooks:
    -   id: tach
```

Note that you should specify the version you are using in the `rev` key.

### Standard install
If you don't already have pre-commit hooks set up, you can run:

```bash
tach install pre-commit
```

The command above will install `tach check` as a pre-commit hook, directly into `.git/hooks/pre-commit`.

If that file already exists, you will need to manually add `tach check` to your existing `.git/hooks/pre-commit` file.
