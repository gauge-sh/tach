# API

## `modguard.Boundary`
A `Boundary` makes all internal members private by default.

`Boundary` accepts no arguments, and has no runtime behavior. It is detected statically by `modguard`.
```python
# project/core/__init__.py
import modguard

modguard.Boundary()
```
### In `__init__.py`
When a `Boundary` appears in `__init__.py`, this marks the contents of the entire package as private by default.
```python
# project/core/inner.py
# This function will be considered private
# due to the boundary at 'project.core'
def private_function():
    ...
```

### In Modules
When a `Boundary` appears in a standalone Python file, this marks the contents of the file itself as private by default.
```python
# project/core/other.py
from modguard import Boundary

Boundary()

# This function will be considered private
# due to the boundary at 'project.core.other'
def other_private_function():
    ...
```

## `modguard.public`
Marking a member as `public` allows it to be imported outside its boundary. This should be used to mark the intended public interface of a package or module.

`public` can be used as either a decorator or a bare function, and has no runtime behavior. It is detected statically by `modguard`.

### Parameters
`public` accepts one optional positional argument (`path`) and one optional keyword argument (`allowlist`).

When present, `path` identifies the object being marked as public.
```python
import modguard

x: int = 3

# These are functionally the same
modguard.public("x")
modguard.public(x)
```

When present, `allowlist` defines a list of module paths which are allowed to import the object. Modules which are descendants of the modules in the `allowlist` are also allowed. If any other modules import the object, they will be flagged as errors by `modguard`.
```python
# In project/utils.py
import modguard

x: int = 3

modguard.public(x, allowlist=["project.core.domain"])

...
# In project/core/other_domain/logic.py
# This import is not allowed,
# because the module ('project.core.other_domain.logic')
# is not contained by any module in the allowlist
from project.utils import x
```

### As a Decorator
`public` can also be used as a decorator to mark functions and classes as public. Its behavior is the same as when used as a function, and it accepts the same keyword arguments (the decorated object is treated as `path`)

```python
import modguard

@modguard.public(allowlist=["project.core.domain"])
def my_pub_function():
    ...
```

### Entire Module
When `public` is used without a `path` argument, it signifies that the entire containing module is public. This means that any top-level member of the module or the module itself can be imported externally (subject to `allowlist`).
```python
# In project/core/logic.py
import modguard

modguard.public()
...
# In project/cli.py
# This import is allowed because 'project.core.logic' is public 
from project.core import logic
```

### In `__init__.py`
When `public` is used without a `path` argument in the `__init__.py` of a package, the top-level module of the package is treated as public.
```python
# In project/core/__init__.py
import modguard

modguard.Boundary()
modguard.public()
...
# In project/cli.py
# This import is allowed because 'project.core' is public 
from project import core
```

## `modguard-ignore`
To ignore a particular import which should be allowed unconditionally, use the `modguard-ignore` comment directive.
```python
# modguard-ignore
from core.main import private_function
```
The directive can also be specific about the import to ignore, which is particularly useful when importing multiple modules.
```python
# modguard-ignore private_function
from core.main import private_function, public_function
```
Note: Names given to `modguard-ignore` should match the alias as it is used in the subsequent import line, not the full module path from the project root.

Note: Boundary violations are detected at the import layer. This means that specific nonstandard custom syntax to access modules/submodules such as getattr or dynamically generated namespaces will not be caught by modguard.