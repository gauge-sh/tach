# Overview

## What is modguard?
Modguard enables you to explicitly define a public interface for your Python modules. Marking a package with a `Boundary` will make all of its internals private by default, exposing only the members marked with `public`.

This enforces an architecture of decoupled and well defined modules, and ensures the communication between domains is only done through their expected public interfaces.

Modguard is incredibly lightweight, and has no impact on the runtime of your code. Instead, its checks are performed through a static analysis CLI tool.

## Commands
* [`modguard init [dir-name]`](usage.md#modguard-init) - Initialize package boundaries in a directory.
* [`modguard check [dir-name]`](usage.md#modguard-check) - Check boundaries are respected throughout a directory.
* [`modguard show [dir-name]`](usage.md#modguard-show) - View and optionally generate a YAML representation of the boundaries in a directory.
