[![image](https://img.shields.io/pypi/v/modguard.svg)](https://pypi.Python.org/pypi/modguard)
[![image](https://img.shields.io/pypi/l/modguard.svg)](https://pypi.Python.org/pypi/modguard)
[![image](https://img.shields.io/pypi/pyversions/modguard.svg)](https://pypi.Python.org/pypi/modguard)
[![image](https://github.com/Never-Over/modguard/actions/workflows/ci.yml/badge.svg)](https://github.com/Never-Over/modguard/actions/workflows/ci.yml)
[![Checked with pyright](https://microsoft.github.io/pyright/img/pyright_badge.svg)](https://microsoft.github.io/pyright/)
[![Ruff](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/astral-sh/ruff/main/assets/badge/v2.json)](https://github.com/astral-sh/ruff)
# modguard
A Python tool to enforce a modular, decoupled package architecture.

![](https://raw.githubusercontent.com/Never-Over/modguard/main/docs/modguard_screencap_2x.gif)

[Docs](https://never-over.github.io/modguard/)

### What is modguard?
Modguard allows you to define boundaries and control dependencies between your Python packages. Each package can also define its public interface.

This enforces a decoupled, modular architecture, and prevents tight coupling.
If a package tries to import from another package that is not listed as a dependency, modguard will throw an exception.
If a package tries to import from another package and does not use its public interface, with `strict: true` set modguard will throw an exception.

Modguard is incredibly lightweight, and has no impact on your runtime. Instead, its checks are performed as a lint check through the CLI.

### Installation
```bash
pip install modguard
```
### Usage
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
Next, specify the constraints for each tag in `modguard.yml` in the root of your project:
```yaml
# [root]/modguard.yml
constraints:
  core:
    depends_on: ["db", "utils"]
  db:
    depends_on: ["utils"]
  utils:
    depends_on: []
```
With these rules in place, packages with tag `core` can import from packages with tag `db` or `utils`. Packages tagged with `db` can only import from `utils`, and packages tagged with `utils` cannot import from any other packages in the project. 

Modguard will now flag any violation of these boundaries.
```bash
# From the root of your Python project (in this example, `project/`)
> modguard check
❌ ./utils/helpers.py: Import "core.PublicAPI" is blocked by boundary "core". Tag(s) ["utils"] do not have access to ["core"].
```

If you want to define a public interface for the package, import and reference each object you want exposed in the package's `__init__.py`:
```python
# db/__init__.py
from db.service import PublicAPI

__all__ = ["PublicAPI"]
```
Turning on `strict: true` in the package's `package.yml` will then enforce that all imports from this package occur through `__init__.py`
```yaml
# db/package.yml
tags: ["db"]
strict: true
```
```python
# The only valid import from "db"
from db import PublicAPI 
```
Modguard will now flag any import that is not from `__init__.py` in the `db` package, in addition to enforcing the dependencies defined above.
```bash
# From the root of your Python project (in this example, `project/`)
> modguard check
❌ ./core/main.py: Import "db.PrivateAPI" is blocked by boundary "db". "db" does not list "db.PrivateAPI" in its public interface.
```

You can also view your entire project's set of dependencies and public interfaces. Run `modguard show` to generate a URL where you can interact with the dependency graph, as well as view your public interfaces:
```bash
> modguard show .
modguard.com/project/id
```
If you want to utilize this data for other purposes, run `modguard show --write .` This will persist the data about your project in a `modguard.json` file.

### Setup
Modguard also comes bundled with a command to set up and define your initial boundaries.
```bash
modguard init .
```
By running `modguard init` from the root of your Python project, modguard will initialize each top-level Python package. Each package will receive a `package.yml` with a single tag based on the folder name. 
The tool will take into consideration the usages between packages, and write a matching set of dependencies to `modguard.yml` in the project root.
```bash
> modguard check
#TODO show passing state here
```

### Advanced
Modguard supports specific exceptions. You can mark an import with the `modguard-ignore` comment:
```python
# modguard-ignore
from db.main import PrivateAPI
```
This will stop modguard from flagging this import as a boundary violation.

You can also specify multiple tags for a given package:
```python
# utils/package.yml
tags: ["core", "utils"]
```
This will expand the set of packages that "utils" can access to include all packages that "core" and "utils" `depends_on` as defined in `modguard.yml`.

`modguard.yml` also accepts regex syntax:
```yaml
    depends_on: [".*"] # Allow imports from anywhere
    depends_on: ["shared.*"] # Allow imports from any package with a tag starting with "shared"
```
By default, modguard ignores hidden directories and files (paths starting with `.`). To override this behavior, set `exclude_hidden_paths` in `modguard.yml`
```yaml
exclude_hidden_paths: false
```

### Details
Modguard works by analyzing the abstract syntax tree (AST) of your codebase. It has no runtime impact, and all operations are performed statically. 

Boundary violations are detected at the import layer. This means that specific nonstandard custom syntax to access packages such as `getattr` or dynamically generated namespaces will not be caught by modguard.

[PyPi Package](https://pypi.org/project/modguard/)

### License
[GNU GPLv3](LICENSE)
