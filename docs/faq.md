# FAQ

### How does it work?
Modguard works by analyzing the abstract syntax tree (AST) of your codebase. The `Boundary` class and `@public` decorator have no runtime impact, and are detected by modguard statically. Boundary violations are detected at import time.

### Why does `modguard` live in my application code?
Modguard is written as a Python library for a few main reasons:
- **Visibility**: When boundary information is co-located with application code, it is visible to a code reviewer or future maintainer.
- **Maintenance**: When packages or public members are moved, renamed, or removed, in-line `modguard` will automatically match the new state (since it will move along with the code, or be removed along with the code).
- **Extensibility**: Having `modguard` in-line will support future dynamic configuration or runtime violation monitoring.

### What is a boundary?
A **boundary** can be thought of as defining a logical module within your project. A project composed of decoupled logical modules with explicit public interfaces is easier to test and maintain.

### Are conditional imports checked?
At the moment, `modguard` will check all imports in your source files, including those which are called conditionally. This is an intentional design decision, but may be made configurable in the future.

### Can you catch dynamic references?
Since `modguard` uses the AST to find imports and public members, dynamic imports (e.g. using a string path) and dynamic names (e.g. using `setattr`, `locals`, `globals`) are generally not supported. If these usages cause `modguard` to report incorrect errors, the [ignore directive](api.md#modguard-ignore) should be sufficient to reach a passing state.
