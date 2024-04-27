# Strict Mode

A module can enable 'strict mode' by setting `strict: true` in the [`module.yml` file](configuration.md#moduleyml).

## How does it work?
When a module is in strict mode, other modules may only import names declared in `__all__` in the `__init__.py` of the package.
This creates an explicit public interface for the module which prevents coupling to implementation details, and makes future changes easier.

## Example

Given modules called 'core' and 'parsing', we may have `module.yml` and `modguard.yml` contents like this:

`core/module.yml`
```yaml
tags: ['core']
strict: true
```

`parsing/module.yml`
```yaml
tags: ['parsing']
```

`modguard.yml`
```yaml
constraints:
  parsing:
    depends_on:
    - core
```

Then, in a file within the 'parsing' module, we might have:
```python
from core.main import get_data  # This import fails

get_data()
```

This import would **fail** `modguard check` with the following error:
```shell
❌ parsing: Module 'core' is in strict mode. Only imports from the root of this module are allowed. The import 'core.main.get_data' (in 'parsing') is not included in __all__.
```

If `get_data` should actually be part of the public interface of 'core', it needs to be specified in `__all__` of `core/__init__.py`:

`core/__init__.py`
```python
from .main import get_data

__all__ = ["get_data"]
```

which would allow 'parsing' to depend on this interface:

```python
from core import get_data  # This import is OK

get_data()
```