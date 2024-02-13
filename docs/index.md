# Overview

## What is modguard?
Modguard enables you to explicitly define the public interface for a given module. Marking a package with a `Boundary` makes all of its internals private by default, exposing only the members marked with the `@public` decorator.

This enforces an architecture of decoupled and well defined modules, and ensures the communication between domains is only done through their expected public interfaces.

Modguard is incredibly lightweight, and has no impact on the runtime of your code. Instead, its checks are performed through a static analysis CLI tool.

## Commands

* `modguard [dir-name]` - Check boundaries are respected throughout a directory.
* `modguard init [dir-name]` - Initialize package boundaries in a directory.
* `modguard show [dir-name]` - Generate a YAML representation of the boundaries in a directory.
