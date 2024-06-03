# Configuration

Aside from running `tach mod` and `tach sync`, you can configure `tach` by creating or modifying the configuration file as described below.

## `tach.yml`

This is the project-level configuration file which should be in the root of your project.

`modules` defines the modules in your project, and accepts the keys described [below.](#modules)

`exclude` accepts a list of directory patterns to exclude from checking.

`ignore_type_checking_imports` is a boolean which, when enabled, silences `tach check` failures caused by imports under a `TYPE_CHECKING` conditional block


```yaml
modules:
- path: tach
  depends_on: []
  strict: true
- path: tach.cache
  depends_on:
  - tach.filesystem
  strict: true
- path: tach.filesystem
  depends_on: []
  strict: true
exclude:
- .*__pycache__/
- build/
- dist/
- docs/
- tach.egg-info/
- tests/
exact: true
disable_logging: false
ignore_type_checking_imports: true
```

## modules
Each module listed under the `modules` key above can accept the following attributes:

`path` should be the Python import path to the module (e.g. `a.b` for `<root>/a/b.py`)

`depends_on` should be a list of the other modules which the module is allowed to import from, using their 'paths' to identify them

`strict` enables [strict mode](strict-mode.md) for the module