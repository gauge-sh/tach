# Overview

## What is tach?
`tach` allows you to define boundaries and control dependencies between your Python packages.
Packages can also define an explicit public interface through `__all__` to prevent deep coupling.

This enforces a decoupled, modular architecture, which makes maintenance and development easier.

If a package tries to import from another package that is not listed as a dependency, `tach` will throw an exception.

When a package is in ['strict mode'](strict-mode.md), if another package tries to import from it without using its public interface, `tach` will throw an exception.

`tach` runs on the CLI, and is ideal for pre-commit hooks and CI checks.

Tach is:
- 🌎 Open source
- 🐍 Installable via pip
- 🔧 Able to be adopted incrementally
- ⚡  Implemented with no runtime impact
- ♾️ Interoperable with your existing systems (cli, hooks, ci, etc.)

## Commands
* [`tach pkg`](usage.md#tach-pkg) - Interactively define package boundaries in your Python project.
* [`tach check`](usage.md#tach-check) - Check that boundaries are respected.
* [`tach sync`](usage.md#tach-sync) - Sync constraints with actual dependencies in your Python project.
* [`tach install`](usage.md#tach-install) - Install `tach` into your development workflow (e.g. pre-commit)
* [`tach clean`](usage.md#tach-clean) - Delete all existing configuration and start from a clean slate.

