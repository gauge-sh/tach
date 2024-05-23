# Usage

## tach check
`tach` will flag any unwanted imports between packages. We recommend you run `tach check` like a linter or test runner, e.g. in pre-commit hooks, on-save hooks, and in CI pipelines.

```bash
usage: tach check [-h] [--exact] [-e file_or_path,...]

Check existing boundaries against your dependencies and package interfaces

options:
  -h, --help            show this help message and exit
  --exact               Raise errors if any dependency constraints are unused.
  -e file_or_path,..., --exclude file_or_path,...
                        Comma separated path list to exclude. tests/, ci/, etc.
```

An error will indicate:

- the file path in which the error was detected
- the tags associated with that file
- the tags associated with the attempted import

If `--exact` is provided, additional errors will be raised if a dependency exists in `tach.yml` that is not exercised by the code.

Example:
```bash
# From the root of your Python project (in this example, `project/`)
> tach check
❌ utils/helpers.py[L10]: Cannot import 'core.PublicAPI'. Tags ['utils'] cannot depend on ['core'].
```

NOTE: If your terminal supports hyperlinks, you can click on the failing file path to go directly to the error.


## tach pkg
`tach` comes bundled with a command to set up and define your initial boundaries.

```bash
usage: tach pkg [-h] [-d [DEPTH]] [-e file_or_path,...]

Configure package boundaries interactively

options:
  -h, --help            show this help message and exit
  -d [DEPTH], --depth [DEPTH]
                        The number of child directories to search for packages to auto-select
  -e file_or_path,..., --exclude file_or_path,...
                        Comma separated path list to exclude. tests/, ci/, etc.
```

Running `tach pkg` will open an interactive editor in your terminal which allows you to mark your package boundaries.

You can navigate with the arrow keys, mark individual packages with `Enter`, and mark all sibling directories
as packages with `Ctrl + a`.

If you have not configured `tach` in your project before, `tach pkg` will automatically mark an initial set of packages.
The `--depth` flag controls how many directories `tach` will traverse when suggesting these initial packages.
You can accept these suggestions immediately with `Ctrl + s`, or you can edit the selections freely before confirming.

Any time you make changes with `tach pkg`, it is recommended to run [`tach sync`](usage.md#tach-sync)
to automatically configure dependency rules.

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

When this command runs, `tach` will analyze the imports in your packages.

Any undeclared dependencies or other dependency errors will be automatically resolved by
adding the corresponding dependencies to your `tach.yml` file.

If you supply `--prune`,
any dependency constraints in your `tach.yml` which are not necessary will also be removed.

## tach clean
If you ever want to remove all configuration for `tach` and start over, you can use `tach clean`:

```bash
usage: tach clean [-h] [--force]

Delete existing configuration and start from an empty slate.

options:
  -h, --help  show this help message and exit
  --force     Do not prompt for confirmation.
```

This will find the nearest `tach` project in parent directories, or simply work from the current directory.
It will remove `tach.yml` along with any `package.yml` files it finds.

## tach install
`tach` can be installed into your development workflow automatically as a pre-commit hook.


### With pre-commit framework
If you use the [pre-commit framework](https://github.com/pre-commit/pre-commit), you can add the following to your `.pre-commit-hooks.yaml`:

```yaml
repos:
-   repo: https://github.com/gauge-sh/tach
    rev: v0.2.4  # change this to the latest tag!
    hooks:
    -   id: tach
        # args: ["--root=backend_root"]
```

Note that you should specify the version you are using in the `rev` key.

Using `args`, you can specify the root of your Python project, where `tach check` should run.
This path should be relative to your git root.

### Standard install
If you don't already have pre-commit hooks set up, you can run:

```bash
tach install pre-commit
```

The command above will install `tach check` as a pre-commit hook, directly into `.git/hooks/pre-commit`.

If that file already exists, you will need to manually add `tach check` to your existing `.git/hooks/pre-commit` file.
