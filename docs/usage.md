# Usage

## modguard check
`modguard` will flag any unwanted imports between modules. It is recommended to run `modguard check` in a similar way as a linter or test runner, e.g. in pre-commit hooks, on-save hooks, and in CI pipelines.

```bash
usage: modguard check [-h] [-e file_or_path,...]

Check boundaries with modguard

options:
  -h, --help            show this help message and exit
  -e file_or_path,..., --exclude file_or_path,...
                        Comma separated path list to exclude. tests/, ci/, etc.
```


## modguard init
`modguard` comes bundled with a command to set up and define your initial boundaries.

By running `modguard init` from the root of your Python project, `modguard` will create an initial set of `module.yml` files to identify your Python packages as modules.

These initial modules will receive a single 'tag' based on their path from the project root. The modules will _not_ be in strict mode by default, but setting `strict: true` in the `module.yml` file will enable this. See ['Strict Mode'](strict-mode.md) for details.

In addition to their tags, the `module.yml` files will contain a `depends_on` key which includes all the dependencies that `modguard` was able to detect automatically for the module, which means that after running `modguard init`, your project will be in a permissive, but passing state.

```bash
usage: modguard init [-h] [-e file_or_path,...]

Initialize boundaries with modguard

options:
  -h, --help            show this help message and exit
  -e file_or_path,..., --exclude file_or_path,...
                        Comma separated path list to exclude. tests,ci,...
```
