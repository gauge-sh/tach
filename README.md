[![image](https://img.shields.io/pypi/v/tach.svg)](https://pypi.Python.org/pypi/tach)
[![image](https://img.shields.io/pypi/l/tach.svg)](https://pypi.Python.org/pypi/tach)
[![image](https://img.shields.io/pypi/pyversions/tach.svg)](https://pypi.Python.org/pypi/tach)
[![image](https://github.com/Never-Over/tach/actions/workflows/ci.yml/badge.svg)](https://github.com/Never-Over/tach/actions/workflows/ci.yml)
[![Checked with pyright](https://microsoft.github.io/pyright/img/pyright_badge.svg)](https://microsoft.github.io/pyright/)
[![Ruff](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/astral-sh/ruff/main/assets/badge/v2.json)](https://github.com/astral-sh/ruff)
# tach
a Python tool to enforce modular design


[Docs](https://never-over.github.io/tach/)

[Discord](https://discord.gg/7crTTJwDv9) - come say hi!


 <video loop src="https://github.com/Never-Over/tach/assets/10570340/a9d8d4df-d262-4b2b-b69a-adbc30d069aa">Tach Demo</video> 


## What is tach?
`tach` allows you to define boundaries and control dependencies between your Python packages. Each package can also define its public interface.

This enforces a decoupled, modular architecture, and prevents tight coupling.
If a package tries to import from another package that is not listed as a dependency, tach will report an error.
If a package tries to import from another package and does not use its public interface, with `strict: true` set, `tach` will report an error.

`tach` is incredibly lightweight, and has no impact on your runtime. Instead, its checks are performed as a lint check through the CLI.

## Installation
```bash
pip install tach
```

## Defining Packages
To define a package, add a `package.yml` to the corresponding Python package. Add at least one 'tag' to identify the package:
```python
# core/package.yml
tags: ["core"]
```
```python
# db/package.yml
tags: ["db"]
```
```python
# utils/package.yml
tags: ["utils"]
```
Next, specify the constraints for each tag in `tach.yml` in the root of your project:
```yaml
# [root]/tach.yml
constraints:
- tag: core
  depends_on:
  - db
  - utils
- tag: db
  depends_on:
  - utils
- tag: utils
  depends_on: []
```
With these rules in place, packages with tag `core` can import from packages with tag `db` or `utils`. Packages tagged with `db` can only import from `utils`, and packages tagged with `utils` cannot import from any other packages in the project. 

`tach` will now flag any violation of these boundaries.
```bash
# From the root of your Python project (in this example, `project/`)
> tach check
❌ ./utils/helpers.py: Import "core.PublicAPI" is blocked by boundary "core". Tag(s) ["utils"] do not have access to ["core"].
```

## Defining Interfaces
If you want to define a public interface for the package, import and reference each object you want exposed in the package's `__init__.py` and add its name to `__all__`:
```python
# db/__init__.py
from db.service import PublicAPI

__all__ = ["PublicAPI"]
```
Turning on `strict: true` in the package's `package.yml` will then enforce that all imports from this package occur through `__init__.py` and are listed in `__all__`
```yaml
# db/package.yml
tags: ["db"]
strict: true
```
```python
# The only valid import from "db"
from db import PublicAPI 
```

## Initial Setup
`tach` also comes bundled with a command to set up and define your initial boundaries.
```bash
tach init
```
By running `tach init` from the root of your Python project, `tach` will initialize each top-level Python package. Each package will receive a `package.yml` with a single tag based on the folder name. 
The tool will take into consideration the usages between packages, and write a matching set of dependencies to `tach.yml` in the project root.

If you'd like to incrementally or individually add new packages to your `tach.yml`, you can use:
```bash
tach add [file_or_path]
```
This will create a boundary around the given file or directory, and update your `tach.yml` with the correct set of dependencies.


### Pre-Commit Hook
`tach` can be installed as a pre-commit hook. See the [docs](https://never-over.github.io/tach/usage/#tach-install) for installation instructions.


## Advanced
`tach` supports specific exceptions. You can mark an import with the `tach-ignore` comment:
```python
# tach-ignore
from db.main import PrivateAPI
```
This will stop `tach` from flagging this import as a boundary violation.

You can also specify multiple tags for a given package:
```python
# utils/package.yml
tags: ["core", "utils"]
```
This will expand the set of packages that "utils" can access to include all packages that "core" and "utils" `depends_on` as defined in `tach.yml`.

`tach.yml` also accepts regex patterns:
```yaml
    depends_on: [".*"] # Allow imports from anywhere
    depends_on: ["shared.*"] # Allow imports from any package with a tag starting with "shared"
```
By default, `tach` ignores hidden directories and files (paths starting with `.`). To override this behavior, set `exclude_hidden_paths` in `tach.yml`
```yaml
exclude_hidden_paths: false
```

## Details
`tach` works by analyzing the abstract syntax tree (AST) of your codebase. It has no runtime impact, and all operations are performed statically. 

Boundary violations are detected at the import layer. This means that dynamic imports using `importlib` or similar approaches will not be caught by tach.

[PyPi Package](https://pypi.org/project/tach/)

### License
[GNU GPLv3](LICENSE)
