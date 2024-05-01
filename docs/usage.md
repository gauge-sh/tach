# Usage

## tach check
`tach` will flag any unwanted imports between packages. We recommend you run `tach check` like a linter or test runner, e.g. in pre-commit hooks, on-save hooks, and in CI pipelines.

```bash
usage: tach check [-h] [-e file_or_path,...]

Check boundaries with tach

options:
  -h, --help            show this help message and exit
  -e file_or_path,..., --exclude file_or_path,...
                        Comma separated path list to exclude. tests/, ci/, etc.
```


## tach init
`tach` comes bundled with a command to set up and define your initial boundaries.

```bash
usage: tach init [-h] [-e file_or_path,...] [-d depth]

Initialize boundaries with tach

options:
  -h, --help            show this help message and exit
  -d [depth], --depth [depth]
                        The number of child directories to search for packages to initialize
  -e file_or_path,..., --exclude file_or_path,...
                        Comma separated path list to exclude. tests,ci,...
```

By running `tach init` from the root of your Python project, `tach` will create an initial set of `package.yml` files to identify your Python packages.

These initial packages will receive a single 'tag' based on their path from the project root. The packages will _not_ be in strict mode by default, but setting `strict: true` in the `package.yml` file will enable this. See ['Strict Mode'](strict-mode.md) for details.

In addition to their tags, the `package.yml` files will contain a `depends_on` key which includes all the dependencies that `tach` was able to detect automatically for the package, which means that after running `tach init`, your project will be in a permissive, but passing state.

If `tach init` detects a only a single package in the root of your project where it's run, it will set boundaries within that package as well. Otherwise, it will simply create boundaries and dependencies for all top-level packages. You can optionally specify `-d/--depth`, which will create packages up to the specified number. 


## tach add
`tach` also comes with a convenient command to add new packages and dependencies automatically.
```bash 
usage: tach add [-h] [-t tag,...] file_or_path,...

Initialize boundaries between top-level modules and write dependencies to `tach.yml`

positional arguments:
  file_or_path,...      The path(s) of the file or directory to create a module boundary around. Use a comma-separated list for multiple.

options:
  -h, --help            show this help message and exit
  -t tag,..., --tags tag,...
                        The tag for the module to be initialized with. Use a comma-separated list for multiple.
```
`tach add` supports single or multiple paths:
```bash
tach add service.py
tach add service.py,utils 
```
You can also optionally specify the tags you would like the new packages to have:
```bash
tach add utils -t shared
```
`tach` will take each file or directory and turn it into a python package with a `package.yml`. It will also update your `tach.yml` and attempt to get you to a passing state.

Note that if you have [`strict:True`](strict-mode.md) set, you may end up in a failing state due to imports that used to be within a module now crossing a package boundary.

If you have relative imports in a file that is turned into a package, they will also likely break due to the creation of the new package. Both issues should be easily solved by hand.

## tach install
`tach` can be installed into your development workflow automatically as a pre-commit hook. This means `tach check` will run from the root of your repo before any commit is created.


### With pre-commit framework
If you use the [pre-commit framework](https://github.com/pre-commit/pre-commit), you can add the following to your `.pre-commit-hooks.yaml`:

```yaml
repos:
-   repo: https://github.com/Never-Over/tach
    rev: v0.1.2
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