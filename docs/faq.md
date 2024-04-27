# FAQ

### How does it work?
`modguard` works by analyzing the abstract syntax tree (AST) of your codebase. For each import found in your project, `modguard` determines whether the import violates any of the declared dependency rules in `modguard.yml`.

### What is a module?
A **module** in the `modguard` sense is distinct from a typical Python module. A `modguard` module is defined by the presence of a `module.yml` file within a Python package, and can be thought of as a logical boundary within your project.
A project composed of decoupled logical modules with explicit public interfaces is easier to test and maintain.

### Are conditional imports checked?
At the moment, `modguard` will check all imports in your source files, including those which are called conditionally. This is an intentional design decision, but may be made configurable in the future.

### Can you catch dynamic references?
Since `modguard` uses the AST to find imports and public members, dynamic imports (e.g. using a string path) and dynamic names (e.g. using `setattr`, `locals`, `globals`) are generally not supported. If these usages cause `modguard` to report incorrect errors, the [ignore directive](modguard-ignore#modguard-ignore) should be sufficient to reach a passing state.
