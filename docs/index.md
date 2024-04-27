# Overview

## What is modguard?
`modguard` allows you to define boundaries and control dependencies between your Python modules.
Modules can also define an explicit public interface through `__all__` to prevent deep coupling.

This enforces a decoupled, modular architecture, which makes maintenance and feature development easier.

If a module tries to import from another module that is not listed as a dependency, `modguard` will throw an exception.

When a module is in 'strict mode', if another module tries to import from it without using its public interface, `modguard` will throw an exception.

`modguard` runs on the CLI, and is ideal for pre-commit hooks and CI checks.

## Commands
* [`modguard init`](usage.md#modguard-init) - Initialize package boundaries in your Python project.
* [`modguard check`](usage.md#modguard-check) - Check boundaries are respected throughout your Python project.
