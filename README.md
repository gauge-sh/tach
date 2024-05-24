[![image](https://img.shields.io/pypi/v/tach.svg)](https://pypi.Python.org/pypi/tach)
[![image](https://img.shields.io/pypi/l/tach.svg)](https://pypi.Python.org/pypi/tach)
[![image](https://img.shields.io/pypi/pyversions/tach.svg)](https://pypi.Python.org/pypi/tach)
[![image](https://github.com/gauge-sh/tach/actions/workflows/ci.yml/badge.svg)](https://github.com/gauge-sh/tach/actions/workflows/ci.yml)
[![Checked with pyright](https://microsoft.github.io/pyright/img/pyright_badge.svg)](https://microsoft.github.io/pyright/)
[![Ruff](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/astral-sh/ruff/main/assets/badge/v2.json)](https://github.com/astral-sh/ruff)
# Tach
a Python tool to enforce modular design

[Docs](https://gauge-sh.github.io/tach/)


Tach lets you define and enforce dependencies across Python packages in your project. A Python package is any directory that contains an `__init__.py`.

This enforces a decoupled, modular architecture, which makes maintenance and development easier. If a package tries to import from another package that is not listed as a dependency, `tach` will throw an exception.


Here's an example:

![tach_demo_ds](https://github.com/gauge-sh/tach/assets/5150563/c693da70-6f5d-417c-968e-4d0507d957c0)


Tach is:
- 🌎 Open source
- 🐍 Installable via pip
- 🔧 Able to be adopted incrementally
- ⚡  Implemented with no runtime impact
- ♾️ Interoperable with your existing systems (cli, hooks, ci, etc.)

## Getting Started

### Installation
```bash
pip install tach
```
### Setup
Tach allows you to configure what is and is not considered a package. By default, Tach will identify and create configuration for all top level packages it finds. 

You can do this interactively! From the root of your python project, run:
```bash
 tach pkg
# Up/Down: Navigate  Ctrl + Up: Jump to parent  Right: Expand  Left: Collapse
# Ctrl + c: Exit without saving  Ctrl + s: Save packages  Enter: Mark/unmark package  Ctrl + a: Mark/unmark all siblings
```
Mark and unmark each package as needed, depending on what you want to define boundaries for.

Once you have marked all the packages you want to enforce constraints between, run:
```bash
tach sync
```
This will create the root configuration for your project, `tach.yml`, with the dependencies that currently exist between each package you've marked.

You can then see what Tach has found by viewing the `tach.yml`'s contents: 
```
cat tach.yml
```

Note: Dependencies on code that are not marked as packages are out of the scope of Tach and will not be enforced.

### Enforcement
Tach comes with a simple cli command to enforce the boundaries that you just set up! From the root of your Python project, run:
```bash
tach check
```
You will see:
```bash
✅ All package dependencies validated!
```

You can validate that Tach is working by either commenting out an item in a `depends_on` key in `tach.yml`, or by adding an import between packages that didn't previously import from each other. 

Give both a try and run `tach check` again. This will generate an error:
```bash
❌ path/file.py[LNO]: Cannot import 'path.other'. Tags ['scope:other'] cannot depend on ['scope:file']. 
```

### Extras

If an error is generated that is an intended dependency, you can sync your actual dependencies with `tach.yml`:
```bash
tach sync
```
After running this command, `tach check` will always pass.

If your configuration is in a bad state, from the root of your python project you can run: 
```bash
tach clean
```
This will wipe all the configuration generated and enforced by Tach.


Tach also supports:
- [Manual file configuration](https://gauge-sh.github.io/tach/configuration/)
- [Strict public interfaces for packages](https://gauge-sh.github.io/tach/strict-mode/)
- [Inline exceptions](https://gauge-sh.github.io/tach/tach-ignore/)
- [Pre-commit hooks](https://gauge-sh.github.io/tach/usage/#tach-install)


More info in the [docs](https://gauge-sh.github.io/tach/). Tach logs anonymized usage statistics which are easily [opted out](https://gauge-sh.github.io/tach/faq/) of.
If you have any feedback, we'd love to talk!

[Discord](https://discord.gg/a58vW8dnmw)
