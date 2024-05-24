# Configuration

Aside from running `tach pkg` and `tach sync`, you can configure `tach` by creating or modifying the files described below.

## `tach.yml`

This is the project-level configuration file which should be in the root of your project.

`constraints` defines the expected dependencies between tags in your project, and accepts a list of constraints as shown below

`exclude` accepts a list of directory patterns to exclude from checking.

`ignore_type_checking_imports` is a boolean which, when enabled, silences `tach check` failures caused by imports under a `TYPE_CHECKING` conditional block


```yaml
constraints:
- tag: scope:filesystem
  depends_on:
  - scope:utils
- tag: scope:parsing
  depends_on:
  - scope:core
  - scope:filesystem
  - scope:utils
- tag: scope:root
  depends_on:
  - scope:utils
  - scope:core
  - scope:filesystem
  - scope:parsing
exclude:
- tests/
- docs/
- build/
ignore_type_checking_imports: true
```


## `package.yml`

This is the package-level configuration file which should exist in each package in your project.

`tags` accepts a list of string tags which are checked against project-level `constraints`

`strict` accepts a boolean which enables ['Strict Mode'](strict-mode.md) for the package.

```yaml
tags: ['scope:utils']
strict: true
```
