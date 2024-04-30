# FAQ

### How does it work?
`tach` works by analyzing the imports in your packages.
When you define constraints in your project-level `tach.yml`, running `tach check` will verify that the imports in your packages don't create unwanted dependencies.

### What is a constraint?
A constraint is a rule written into your `tach.yml` which specifies how packages can depend on each other, based on their tags.
For example, you can write a constraint which prevents a shared `utility` package from depending on your `core` application code.

### Are conditional imports checked?
At the moment, `tach` will check all imports in your source files, including those which are called conditionally. This is an intentional design decision, but may be made configurable in the future.

### Can you catch dynamic references?
Since `tach` uses the AST to find imports and public members, dynamic imports (e.g. using a string path) and dynamic names (e.g. using `setattr`, `locals`, `globals`) are generally not supported. If these usages cause `tach` to report incorrect errors, the [ignore directive](tach-ignore#tach-ignore) should be sufficient to reach a passing state.
