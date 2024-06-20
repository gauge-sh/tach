# Overview

## What is tach?
`tach` allows you to define boundaries and control dependencies between your Python modules.
Modules can also define an explicit public interface through `__all__` to prevent deep coupling.

This enforces a decoupled, modular architecture, which makes maintenance and development easier.

If a module tries to import from another module that is not listed as a dependency, `tach` will report an error.

When a module is in ['strict mode'](strict-mode.md), if another module tries to import from it without using its public interface, `tach` will report an error.

`tach` runs on the CLI, and is ideal for pre-commit hooks and CI checks.

Tach is:

- 🌎 Open source
- 🐍 Installable via pip
- 🔧 Able to be adopted incrementally
- ⚡ Implemented with no runtime impact
- ♾️ Interoperable with your existing systems (cli, hooks, ci, etc.)

## Commands
* [`tach mod`](usage.md#tach-mod) - Interactively define module boundaries in your Python project.
* [`tach check`](usage.md#tach-check) - Check that boundaries are respected.
* [`tach sync`](usage.md#tach-sync) - Sync constraints with actual dependencies in your Python project.
* [`tach report`](usage.md#tach-report) - Generate a dependency report for a file or module in your Python project.
* [`tach install`](usage.md#tach-install) - Install `tach` into your development workflow (e.g. pre-commit)
