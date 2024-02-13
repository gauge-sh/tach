# Usage

## modguard check
Modguard will flag any unwanted imports between modules. It is recommended to run `modguard check` in a similar way as a linter or test runner, e.g. in pre-commit hooks, on-save hooks, and in CI pipelines.

```bash
usage: modguard check [-h] [-e file_or_path,...] path

Check boundaries with modguard

positional arguments:
  path                  The path of the root of your Python project.

options:
  -h, --help            show this help message and exit
  -e file_or_path,..., --exclude file_or_path,...
                        Comma separated path list to exclude. tests/, ci/, etc.
```


## modguard init
Modguard comes bundled with a command to set up and define your initial boundaries.

By running `modguard init .` from the root of your python project, modguard will declare boundaries on each python package within your project. Additionally, each member of that package which is imported from outside the boundary will be marked `public`. 

This will automatically lock-in the public interface for each package within your project, and instantly reach a passing state when running `modguard`
```bash
usage: modguard init [-h] [-e file_or_path,...] path

Initialize boundaries with modguard

positional arguments:
  path                  The path of the root of your Python project.

options:
  -h, --help            show this help message and exit
  -e file_or_path,..., --exclude file_or_path,...
                        Comma separated path list to exclude. tests/, ci/, etc.
```


## modguard show
Modguard can display your current set of boundaries and public interfaces.

By running `modguard show .` from the root of your python project, you can view your full project's file tree annotated with both boundaries (`[B]`) and members that have been defined as public (`[P]`). Optionally, specifying `-w/--write` will write the output to a `modguard.yaml` file, which can then be consumed for external usecases.
```bash
usage: modguard show [-h] [-e file_or_path,...] [-w] path

Show your existing boundaries in modguard

positional arguments:
  path                  The path of the root of your Python project.

options:
  -h, --help            show this help message and exit
  -e file_or_path,..., --exclude file_or_path,...
                        Comma separated path list to exclude. tests/, ci/, etc.
  -w, --write           Write the output to a `modguard.yaml` file
```