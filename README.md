[![PyPI version](https://badge.fury.io/py/modguard.svg)](https://badge.fury.io/py/modguard)

# modguard
A Python tool to support and enforce a modular package architecture within a monorepo.

### What is modguard?
Modguard enables you to explicitly define the public interface for a given module. Marking a package with a `Boundary` makes all of its internals private by default, exposing only the members marked with the `@public` decorator.

This enforces an architecture of decoupled and well defined modules, and ensures the communication between domains is only done through their expected public interfaces.

Modguard is incredibly lightweight, and has no impact on the runtime of your code. Instead, its checks are performed through a static analysis CLI tool.

### Installation
```bash
pip install modguard
```

### Usage
Add a `Boundary` to the `__init__.py` of the module you're creating an interface for.
```python
# core/__init__.py
from modguard import Boundary

Boundary(__name__)
```

Add the `public` decorator to any callable in the module that should be exported.
```python
# core/main.py
from modguard import public

# Adding the decorator here signifies this function is public
@public
def public_function(user_id: int) -> str:
    ...

# This function will be considered private
def private_function():
    ...
```
Modguard will now flag any incorrect dependencies between modules.
```bash
> # From the root of your project
> modguard .
❌ ./utils/helpers.py: Import 'core.main.private_function' in ./utils/helpers.py is blocked by boundary 'core.main'
```

### Advanced Usage
Modguard also supports specific allow lists within the `public()` decorator.
```python
@public(allowlist=['utils.helpers'])
def public_function(user_id: int) -> str:
    ...
```
This will allow for `public_function` to be imported and used in `utils.helpers`, but restrict it's usage elsewhere. 

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
```shell
> # From the root of your project
> modguard .
❌ ./utils/helpers.py: Import 'core.main' in ./utils/helpers.py is blocked by boundary 'core.main'
```

If you expect to be able to import the entire contents of your module, you can declare an entire module as public to avoid this:
```python
# core/main.py
from modguard import public
public()

...
```
This syntax also supports the `allowlist` parameter.


### Details
Modguard works by analyzing the abstract syntax tree (AST) of your codebase. The `Boundary` class and `@public` decorator have no runtime impact, and are detected by modguard statically. Boundary violations are detected at the import layer; specific nonstandard custom syntax to access modules/submodules such as getattr or dynamically generated namespaces may not be caught by modguard.

[PyPi Package](https://pypi.org/project/modguard/)

### License
[GNU GPLv3](LICENSE)
