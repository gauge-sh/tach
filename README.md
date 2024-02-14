[![image](https://img.shields.io/pypi/v/modguard.svg)](https://pypi.python.org/pypi/modguard)
[![image](https://img.shields.io/pypi/l/modguard.svg)](https://pypi.python.org/pypi/modguard)
[![image](https://img.shields.io/pypi/pyversions/modguard.svg)](https://pypi.python.org/pypi/modguard)
[![image](https://github.com/Never-Over/modguard/actions/workflows/ci.yml/badge.svg)](https://github.com/Never-Over/modguard/actions/workflows/ci.yml)
[![Checked with pyright](https://microsoft.github.io/pyright/img/pyright_badge.svg)](https://microsoft.github.io/pyright/)
[![Ruff](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/astral-sh/ruff/main/assets/badge/v2.json)](https://github.com/astral-sh/ruff)
# modguard
A Python tool to support and enforce a modular, decoupled package architecture.

![](https://raw.githubusercontent.com/Never-Over/modguard/main/docs/modguard_screencap.gif)

[Docs](https://never-over.github.io/modguard/)

### What is modguard?
Modguard enables you to explicitly define the public interface for a given module. Marking a package with a `Boundary` makes all of its internals private by default, exposing only the members marked with the `@public` decorator.

This enforces an architecture of decoupled and well-defined modules, and ensures the communication between domains is only done through their expected public interfaces.

Modguard is incredibly lightweight, and has no impact on the runtime of your code. Instead, its checks are performed through a static analysis CLI tool.

### Installation
```bash
pip install modguard
```
### Usage
Add a `Boundary` to the `__init__.py` of the module you're creating an interface for.
```python
# project/core/__init__.py
import modguard

modguard.Boundary()
```
Add the `public` decorator to any callable in the module that should be exported. You can also export individual members by passing them to `public` as function call arguments.
```python
# project/core/main.py
import modguard

# Adding the decorator here signifies this function is public
@modguard.public
def public_function(user_id: int) -> str:
    ...

# This function will be considered private
def private_function():
    ...

PUBLIC_CONSTANT = "Hello world"
# Allow export of PUBLIC_CONSTANT from this module
public(PUBLIC_CONSTANT)
```
Modguard will now flag any incorrect dependencies between modules.
```bash
# From the root of your python project (in this example, `project/`)
> modguard check .
❌ ./utils/helpers.py: Import 'core.main.private_function' in ./utils/helpers.py is blocked by boundary 'core.main'
```
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
By running `modguard init` from the root of your python project, modguard will inspect and declare boundaries on each python package within your project. Additionally, each accessed member of that package will be decorated with `public`.

This will automatically create boundaries and define your public interface for each package within your project, and instantly get your project to a passing state for `modguard .`


### Advanced Usage
Modguard also supports specific allow lists within `public`.
```python
@modguard.public(allowlist=['utils.helpers'])
def public_function(user_id: int) -> str:
    ...

PUBLIC_CONSTANT = "Hello world"
public(PUBLIC_CONSTANT, allowlist=['utils.helpers'])

```
This will allow for `public_function` and `PUBLIC_CONSTANT` to be imported and used in `utils.helpers`, but restrict its usage elsewhere.

Alternatively, you can mark an import with the `modguard-ignore` comment:
```python
# modguard-ignore
from core.main import private_function
```
This will stop modguard from flagging this import as a boundary violation.

Given that python allows for dynamic importing at runtime, modguard will fail if a whole module is imported without being declared public.
```python
from core import main # contains public and private members
```
```bash
# From the root of your project
> modguard .
❌ ./utils/helpers.py: Import 'core.main' in ./utils/helpers.py is blocked by boundary 'core.main'
```

If you expect to be able to import the entire contents of your module, you can declare an entire module as public to avoid this:
```python
# core/main.py
import modguard

modguard.public()

...
```
This syntax also supports the `allowlist` parameter.

### Details
Modguard works by analyzing the abstract syntax tree (AST) of your codebase. The `Boundary` class and `@public` decorator have no runtime impact, and are detected by modguard statically. 

Boundary violations are detected at the import layer. This means that specific nonstandard custom syntax to access modules/submodules such as getattr or dynamically generated namespaces will not be caught by modguard.

[PyPi Package](https://pypi.org/project/modguard/)

### License
[GNU GPLv3](LICENSE)
