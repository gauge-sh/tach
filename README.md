
# Tach

[![downloads](https://static.pepy.tech/badge/tach/month)](https://pepy.tech/project/tach)
[![version](https://img.shields.io/pypi/v/tach.svg)](https://pypi.Python.org/pypi/tach)
[![license](https://img.shields.io/pypi/l/tach.svg)](https://pypi.Python.org/pypi/tach)
[![python](https://img.shields.io/pypi/pyversions/tach.svg)](https://pypi.Python.org/pypi/tach)
[![ci](https://github.com/gauge-sh/tach/actions/workflows/ci.yml/badge.svg)](https://github.com/gauge-sh/tach/actions/workflows/ci.yml)
[![pyright](https://microsoft.github.io/pyright/img/pyright_badge.svg)](https://microsoft.github.io/pyright/)
[![ruff](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/astral-sh/ruff/main/assets/badge/v2.json)](https://github.com/astral-sh/ruff)

a Python tool to enforce modular design, written in Rust.

[Docs](https://gauge-sh.github.io/tach/)

<div align="center">
    <img src="./logo.png" alt="gauge-logo" width="30%"  style="border-radius: 50%; padding-bottom: 20px"/>
</div>

Tach lets you define and enforce dependencies between Python modules in your project.

This enforces a decoupled, modular architecture, which makes maintenance and development easier. If a module tries to import from another module that is not listed as a dependency, `tach` will report an error.


Here's an example:

![tach_demo](https://github.com/gauge-sh/tach/assets/10570340/6fc1e9b4-5a78-4330-a329-9187bd9c374d)

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
Tach allows you to configure where you want to place module boundaries in your project.

You can do this interactively! Run:
```bash
 tach mod
# Up/Down: Navigate  Enter: Mark/unmark module  Right: Expand  Left: Collapse  Ctrl + Up: Jump to parent
# Ctrl + s: Exit and save  Ctrl + c: Exit without saving  Ctrl + a: Mark/unmark all
```
Mark and unmark each module boundary you want to create with 'Enter' (or 'Ctrl + a' to mark all sibling modules). Common choices would be to mark all of your top-level Python source packages, or just a few packages which you want to isolate.

If your Python code lives below your project root, you should mark your Python [source root](https://gauge-sh.github.io/tach/configuration#source-root) using the 's' key.

Once you have marked all the modules you want to enforce constraints between, run:
```bash
tach sync
```
This will create the main configuration file for your project, `tach.yml`, with the dependencies that currently exist between each module you've marked.

You can then see what Tach has found by viewing the `tach.yml`'s contents: 
```
cat tach.yml
```

NOTE: Your 'project root' directory (the directory containing your `tach.yml`) will implicitly be treated as a module boundary, and may show up in your dependency constraints as '<root>'.

### Enforcement
Tach comes with a simple cli command to enforce the boundaries that you just set up! From the root of your Python project, run:
```bash
tach check
```
You will see:
```bash
✅ All module dependencies validated!
```

You can validate that Tach is working by either commenting out an item in a `depends_on` key in `tach.yml`, or by adding an import between modules that didn't previously import from each other. 

Give both a try and run `tach check` again. This will generate an error similar to this one:
```bash
❌ tach/check.py[L8]: Cannot import 'tach.filesystem'. Tag 'tach' cannot depend on 'tach.filesystem'. 
```

Each error indicates an import which violates your module's declared dependencies. If your terminal supports hyperlinks, you can click on the failing file path to go directly to the error.

### Extras

If an error is generated that is an intended dependency, you can sync your actual dependencies with `tach.yml`:
```bash
tach sync
```
After running this command, `tach check` will always pass.


If you just want to see the dependencies and usages of a given file or module in your project, you can use `tach report`.

```bash
tach report my_package/
# OR
tach report my_module.py
```

Tach also supports:
- [Manual file configuration](https://gauge-sh.github.io/tach/configuration/)
- [Strict public interfaces for modules](https://gauge-sh.github.io/tach/strict-mode/)
- [Inline exceptions](https://gauge-sh.github.io/tach/tach-ignore/)
- [Pre-commit hooks](https://gauge-sh.github.io/tach/usage/#tach-install)


More info in the [docs](https://gauge-sh.github.io/tach/). Tach logs anonymized usage statistics which are easily [opted out](https://gauge-sh.github.io/tach/faq/) of.
If you have any feedback, we'd love to talk!

If you have any questions or run into any issues, let us know by either reaching out on [Discord](https://discord.gg/a58vW8dnmw) or submitting a [Github Issue](https://github.com/gauge-sh/tach/issues)!
