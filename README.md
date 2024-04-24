[![image](https://img.shields.io/pypi/v/modguard.svg)](https://pypi.python.org/pypi/modguard)
[![image](https://img.shields.io/pypi/l/modguard.svg)](https://pypi.python.org/pypi/modguard)
[![image](https://img.shields.io/pypi/pyversions/modguard.svg)](https://pypi.python.org/pypi/modguard)
[![image](https://github.com/Never-Over/modguard/actions/workflows/ci.yml/badge.svg)](https://github.com/Never-Over/modguard/actions/workflows/ci.yml)
[![Checked with pyright](https://microsoft.github.io/pyright/img/pyright_badge.svg)](https://microsoft.github.io/pyright/)
[![Ruff](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/astral-sh/ruff/main/assets/badge/v2.json)](https://github.com/astral-sh/ruff)
# modguard
A Python tool to enforce a modular, decoupled package architecture.

![](https://raw.githubusercontent.com/Never-Over/modguard/main/docs/modguard_screencap_2x.gif)

[Docs](https://never-over.github.io/modguard/)

### What is modguard?
Modguard allows you to enforce boundaries and dependencies between your python modules. Each module can also define it's public interface.

This enforces an architecture of decoupled modules, and ensures the communication between domains only happens through their defined public interfaces.
If another module tries to import from a module that is not an explicitly listed dependency, Modguard will throw an exception.
If another module tries to import from a module outside of its public interface, modguard will throw an exception.

Modguard is incredibly lightweight, and has no impact on the runtime of your code. Instead, its checks are performed through as a lint check through its CLI.

### Installation
```bash
pip install modguard
```
### Usage
Add an `interface.py` to the root directory of the module you're creating an interface for. Create a `tag` that will be used to specify module dependencies:
```python
# core/interface.py
__tags__ = ["core"]
```
```python
# db/interface.py
__tags__ = ["db"]
```
```python
# utils/interface.py
__tags__ = ["utils"]
```
Next, specify the dependencies in `modguard.yml` in the root of your project:
```yaml
# [root]/modguard.yml
tags:
  - core:
    depends_on: ["db", "utils"]
  - db:
    depends_on: ["utils"]
  - utils:
    depends_on: []
```
With these rules in place, `core` can import from `db` and `utils`. `db` can only import from `utils`, and `utils` cannot import from any other modules in the project. 

Modguard will now flag any violation of these boundaries.
```bash
# From the root of your python project (in this example, `project/`)
> modguard check .
❌ ./utils/helpers.py: Import "core.PublicAPI" is blocked by boundary "core". Tag(s) ["utils"] do not have access to ["core"].
```

If you want to enforce a public interface for the module, import and reference each object you want exposed:
```python
# db/interface.py
from db.service import PublicAPI

__tags__ = ["db"]
__all__ = ["PublicAPI"]
```
```python3
# The only valid import from "db"
from db.interface import PublicAPI 
```

Modguard will now flag any import that is not in `__all__` in `interface.py`, in addition to enforcing the dependencies defined above. Imports for this module must flow through `interface.py`
```bash
# From the root of your python project (in this example, `project/`)
> modguard check .
❌ ./core/main.py: Import "db.PrivateAPI" is blocked by boundary "db". "db" does not list "db.PrivateAPI" in its public interface.
```

#TODO: Show can utilize ERD diagrams. Look into ERAlchemy, mermaid
You can also view your entire project's set of dependencies and public interfaces. Boundaries will be marked with a `[B]`, and public members will be marked with a `[P]`. Note that a module can be both public and a boundary.
```bash
> modguard show .
example
  [B]core
    main
      [P]public_function
      [P]PUBLIC_CONSTANT
  [P][B]utils
    helpers
```
If you want to utilize this data for other purposes, run `modguard show --write .` This will persist the data about your project in a `modguard.yaml` file.
### Setup
Modguard also comes bundled with a command to set up and define your initial boundaries.
```bash
modguard init .
```
By running `modguard init` from the root of your python project, modguard will inspect and declare boundaries on each python module visible from your project root. Each module will receive an `interface.py` with a single tag based on the folder name. 
The tool will take into consideration the usages between modules, and write a matching set of dependencies to `modguard.yml`.
```bash
modguard check
```

### Advanced
Modguard supports specific exceptions. You can mark an import with the `modguard-ignore` comment:
```python
# modguard-ignore
from db.main import PrivateAPI
```
This will stop modguard from flagging this import as a boundary violation.

You can also specify multiple tags for a given module:
```python
# utils/interface.py
__tags__ = ["core", "utils"]
```
This will expand the set of modules that "utils" can access to include all modules that "core" and "utils" `depends_on` as defined in `modguard.yml`.

`modguard.yml` also accepts regex syntax:
```yaml
    depends_on: [".*"] # Allow imports from anywhere
    depends_on: ["shared.*"] # Allow imports from any module with a tag starting with "shared"
```

### Details
Modguard works by analyzing the abstract syntax tree (AST) of your codebase. `interface.py` has no runtime impact, and are read by modguard statically. 

Boundary violations are detected at the import layer. This means that specific nonstandard custom syntax to access modules/submodules such as getattr or dynamically generated namespaces will not be caught by modguard.

[PyPi Package](https://pypi.org/project/modguard/)

### License
[GNU GPLv3](LICENSE)
