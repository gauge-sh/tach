# Strict Mode

A package can enable 'strict mode' by setting `strict: true` in the [`package.yml` file](configuration.md#packageyml).

## How does it work?
When a package is in strict mode, other packages may only import names declared in `__all__` in the `__init__.py` of the package.
This creates an explicit public interface for the package which prevents coupling to implementation details, and makes future changes easier.

## Example

Given packages called 'core' and 'parsing', we may have `package.yml` and `tach.yml` contents like this:

```yaml
# core/package.yml
tags: ['core']
strict: true
```

```yaml
# parsing/package.yml
tags: ['parsing']
```


```yaml
# tach.yml
constraints:
- tag: parsing
  depends_on:
  - core
```

Then, in a file within the 'parsing' package, we may have:
```python
from core.main import get_data  # This import fails

get_data()
```

This import would **fail** `tach check` with the following error:
```shell
❌ parsing: Package 'core' is in strict mode. Only imports from the root of this package are allowed. The import 'core.main.get_data' (in 'parsing') is not included in __all__.
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
`tach check` will now pass!
```bash
✅ All package dependencies validated!
```