# Usage

## tach check
`tach` will flag any unwanted imports between packages. It is recommended to run `tach check` in a similar way as a linter or test runner, e.g. in pre-commit hooks, on-save hooks, and in CI pipelines.

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

By running `tach init` from the root of your Python project, `tach` will create an initial set of `package.yml` files to identify your Python packages.

These initial packages will receive a single 'tag' based on their path from the project root. The packages will _not_ be in strict mode by default, but setting `strict: true` in the `package.yml` file will enable this. See ['Strict Mode'](strict-mode.md) for details.

In addition to their tags, the `package.yml` files will contain a `depends_on` key which includes all the dependencies that `tach` was able to detect automatically for the package, which means that after running `tach init`, your project will be in a permissive, but passing state.

```bash
usage: tach init [-h] [-e file_or_path,...]

Initialize boundaries with tach

options:
  -h, --help            show this help message and exit
  -e file_or_path,..., --exclude file_or_path,...
                        Comma separated path list to exclude. tests,ci,...
```
