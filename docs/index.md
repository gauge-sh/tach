# Overview

## What is tach?
`tach` allows you to define boundaries and control dependencies between your Python packages.
Packages can also define an explicit public interface through `__all__` to prevent deep coupling.

This enforces a decoupled, modular architecture, which makes maintenance and development easier.

If a package tries to import from another package that is not listed as a dependency, `tach` will throw an exception.

When a package is in ['strict mode'](strict-mode.md), if another package tries to import from it without using its public interface, `tach` will throw an exception.

`tach` runs on the CLI, and is ideal for pre-commit hooks and CI checks.

## Commands
* [`tach init`](usage.md#tach-init) - Initialize package boundaries in your Python project.
* [`tach add`](usage.md#tach-add) - Add a new package around a file or directory to your existing config. 
* [`tach check`](usage.md#tach-check) - Check that boundaries are respected.
* [`tach install`](usage.md#tach-install) - Install `tach` into your development workflow (e.g. pre-commit)

