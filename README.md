[![PyPI version](https://badge.fury.io/py/modguard.svg)](https://badge.fury.io/py/modguard)

# modguard
A Python tool to guard against incorrect usage of python modules.

### What is modguard?
Modguard enables you to explicitly define the public interface for a given module.

This helps prevent other developers unintentionally mis-using code, which can lead to poor performance, security vulnerabilities, bugs, and more.

By declaring a module within modguard, all members of the module will be made private by default. Each member of the interface that you intend to make public can then be exposed through a simple decorator.

Modguard is incredibly lightweight, and has no impact on the runtime of your code. Instead, it's checks are performed through a lightweight CLI tool.
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

Implement the `public` decorator on any part of the interface that is public
```python
# core/main.py
from modguard import public

# Adding the decorator here signifies this function is public
@public
def public_function(user_id: int) -> str:
    ...

# This function will now be considered private
def private_function():
    ...
```
Modguard will now flag any incorrect usages of your interface.
```bash
> # From the root of your project
> guard .
1 error found.
utils/helpers.py:L45-60 E001 Restricted usage of 'core.main.private_function' in 'utils.helpers'
```

### Details
Modguard works by analyzing the abstract syntax tree of your codebase. It will only protect against usages that are within the scope of the cli runtime, which is why we suggest always running the tool from the root of your project.

[PyPi Package](https://pypi.org/project/modguard/)

### License
[GNU GPLv3](LICENSE)
