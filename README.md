[![PyPI version](https://badge.fury.io/py/modguard.svg)](https://badge.fury.io/py/modguard)

# modguard
A Python tool to support and enforce a modular package architecture within a monorepo.

### What is modguard?
Modguard enables you to explicitly define the public interface for a given module. Marking a package with a `modguard.Boundary` makes all of its internals private by default, exposing only the members marked with the `@public` decorator.

This promotes an architecture of decoupled modules, and ensures the communication between domains is only done through their expected public interfaces.

Modguard is incredibly lightweight, and has no impact on the runtime of your code. Instead, its checks are performed through a CLI tool performing static analysis.
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

### Details
Modguard works by analyzing the abstract syntax tree (AST) of your codebase. The `Boundary` class and `@public` decorator have no runtime behavior, but are detected by modguard statically.

[PyPi Package](https://pypi.org/project/modguard/)

### License
[GNU GPLv3](LICENSE)
