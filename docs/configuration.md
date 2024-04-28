# Configuration

Running `modguard init` will create the files below, but you will likely want to configure them further.
The configuration fields are described on this page.


## `modguard.yml`

This is the project-level configuration file which should be in the root of your project.
It accepts `exclude` and `constraints` as top-level keys.

`exclude` accepts a list of directory patterns to exclude from checking.

`constraints` defines the expected dependencies between tags in your project, and accepts a list of dictionaries.

```yaml
constraints:
  scope:filesystem:
    depends_on:
    - scope:utils
  scope:parsing:
    depends_on:
    - scope:filesystem
    - scope:utils
  scope:root:
    depends_on:
    - scope:filesystem
    - scope:parsing
    - scope:utils
ignore: ['tests', 'docs']
```


## `package.yml`

This is the package-level configuration file which should exist in each package in your project.
It accepts `tags` and `strict` as top-level keys.

`tags` accepts a list of string tags which map onto project-level `constraints`

`strict` accepts a boolean which enables ['Strict Mode'](strict-mode.md) for the package

```yaml
tags: ['scope:utils']
strict: true
```
