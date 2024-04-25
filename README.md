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
Modguard allows you to enforce boundaries and dependencies between your Python modules. Each module can also define it's public interface.

This enforces an architecture of decoupled modules, and avoids modules becoming tightly intertwined.
If a module tries to import from another module that is not listed as a dependency, modguard will throw an exception.
If a module tries to import from another module and does not use its public interface, with `strict: true` set modguard will throw an exception.

Modguard is incredibly lightweight, and has no impact on your runtime. Instead, its checks are performed through as a lint check through the CLI.

### Installation
```bash
pip install modguard
```
### Usage
Add a `module.yml` to the root directory of each module you're creating a boundary for. Create a `tag` that will be used to specify module dependencies:
python
# core/module.yml
tags: ["core"]
```
python
# db/module.yml
tags: ["db"]
```
python
# utils/module.yml
tags: ["utils"]
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
# From the root of your Python project (in this example, `project/`)
> modguard check
❌ ./utils/helpers.py: Import "core.PublicAPI" is blocked by boundary "core". Tag(s) ["utils"] do not have access to ["core"].
```

If you want to enforce a public interface for the module, import and reference each object you want exposed in the module's `__init__.py`:
python
# db/__init__.py
from db.service import PublicAPI

__all__ = ["PublicAPI"]
```
Turning on `strict: true` in the module's `module.yml` will then enforce that all imports from this module occur through `__init__.py`
```yaml
# db/module.yml
tags: ["db"]
strict: true
```
python3
# The only valid import from "db"
from db import PublicAPI 
```
Modguard will now flag any import that is not from `__init__.py` in the `db` module, in addition to enforcing the dependencies defined above.
```bash
# From the root of your Python project (in this example, `project/`)
> modguard check
❌ ./core/main.py: Import "db.PrivateAPI" is blocked by boundary "db". "db" does not list "db.PrivateAPI" in its public interface.
```

You can also view your entire project's set of dependencies and public interfaces. Run `modguard show` to generate a url where you can interact with the dependency graph, as well as view your public interfaces:
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
By running `modguard init` from the root of your Python project, modguard will inspect and declare boundaries on each Python module visible from your project root. Each module will receive an `interface.py` with a single tag based on the folder name. 
The tool will take into consideration the usages between modules, and write a matching set of dependencies to `modguard.yml`.
```bash
> modguard check
#TODO show passing state here
```

### Advanced
Modguard supports specific exceptions. You can mark an import with the `modguard-ignore` comment:
python
# modguard-ignore
from db.main import PrivateAPI
```
This will stop modguard from flagging this import as a boundary violation.

You can also specify multiple tags for a given module:
python
# utils/module.yml
tags: ["core", "utils"]
```
This will expand the set of modules that "utils" can access to include all modules that "core" and "utils" `depends_on` as defined in `modguard.yml`.

`modguard.yml` also accepts regex syntax:
```yaml
    depends_on: [".*"] # Allow imports from anywhere
    depends_on: ["shared.*"] # Allow imports from any module with a tag starting with "shared"
```

### Details
Modguard works by analyzing the abstract syntax tree (AST) of your codebase. It has no runtime impact, and all operations are performed statically. 

Boundary violations are detected at the import layer. This means that specific nonstandard custom syntax to access modules/submodules such as getattr or dynamically generated namespaces will not be caught by modguard.

[PyPi Package](https://pypi.org/project/modguard/)

### License
[GNU GPLv3](LICENSE)
