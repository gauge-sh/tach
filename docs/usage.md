# Usage

## modguard
Modguard will flag any unwanted imports between modules. It is recommended to run `modguard` in a similar way as a linter or test runner, e.g. in pre-commit hooks, on-save hooks, and in CI pipelines.

```
usage: modguard [-h] [-e file_or_path,...] path

positional arguments:
  path                  The path of the root of your project that contains all defined boundaries.

options:
  -h, --help            show this help message and exit
  -e file_or_path,..., --exclude file_or_path,...
                        Comma separated path list to exclude. tests/,ci/,etc.

Make sure modguard is run from the root of your repo that a directory is being specified. For example: `modguard .`
```


## modguard init
Modguard comes bundled with a command to set up and define your initial boundaries.

By running `modguard init` from the root of your python project, modguard will declare boundaries on each python package within your project. Additionally, each member of that package which is imported from outside the boundary will be marked `public`. 

This will automatically lock-in the public interface for each package within your project, and instantly reach a passing state when running `modguard`
```
usage: modguard init [-h] [-e file_or_path,...] path

Initialize boundaries in a repository with modguard

positional arguments:
  path                  The path of the Python project in which boundaries should be initialized.

options:
  -h, --help            show this help message and exit
  -e file_or_path,..., --exclude file_or_path,...
                        Comma separated path list to exclude. tests/,ci/,etc.
```
