# FAQ

### How does it work?
`tach` works by analyzing the imports in your Python modules.
When you define module dependencies in your project-level `tach.yml`, running `tach check` will verify that the imports in your modules match your expected dependencies.

### What is a module?
A 'module' is a Python module with dependencies configured in `tach.yml`.
The module is identified by its import path from your project root (e.g. `a.b` for `<root>/a/b.py`),
and its dependencies are listed in the `depends_on` key containing module paths in the same format.

### Are conditional imports checked?
At the moment, `tach` will check all imports in your source files, including those which are called conditionally.
The only exceptions are imports made within `TYPE_CHECKING` conditional blocks. If you want to disable checks for
these imports, you can add `ignore_type_checking_imports: true` to your `tach.yml`.

### Can you catch dynamic references?
Since `tach` uses the AST to find imports and public members, dynamic imports (e.g. using a string path) and dynamic names (e.g. using `setattr`, `locals`, `globals`) are generally not supported. If these usages cause `tach` to report incorrect errors, the [ignore directive](tach-ignore.md#tach-ignore) should be sufficient to reach a passing state.


### How can I make a feature request or file a bug?

This project uses [GitHub Issues](https://github.com/gauge-sh/tach/issues) to track bugs and feature requests. Please search the existing
issues before filing new issues to avoid duplicates. File your bug or
feature request as a new Issue.

### How can I get help?

For help and questions, join the [Discord](https://discord.gg/a58vW8dnmw)!


### How can I report a security issue?

Please do not report security vulnerabilities through public GitHub issues. Instead, please send a  DM to caelean or nashsando on [Discord](https://discord.gg/a58vW8dnmw). You can also email us at caelean@gauge.sh or evan@gauge.sh. If the issue is significant, we will consider paying a bounty.

### What information does Tach track?

Tach tracks anonymized usage and error report statistics; we ascribe to Posthog's approach as detailed [here](https://posthog.com/blog/open-source-telemetry-ethical).
If you would like to opt out of sending anonymized info, you can set `disable_logging` to `true` in your `tach.yml`.



