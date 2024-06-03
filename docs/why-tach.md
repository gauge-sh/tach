# Why `tach`?

## The Problem
By default, Python allows you to import and use anything, anywhere. Over time, this results in modules that were intended to be separate getting tightly coupled together, and domain boundaries breaking down. We experienced this first-hand at a unicorn startup, where the entire engineering team paused development for over a year in an attempt to split up tightly coupled packages into independent micro-services. This ultimately failed, and resulted in the CTO getting fired.

This problem occurs because:

- It's much easier to add to an existing package rather than create a new one
- Junior devs have a limited understanding of the existing architecture
- External pressure leading to shortcuts and overlooking best practices

Attempts we've seen to fix this problem always came up short. A patchwork of solutions would attempt to solve this from different angles, such as developer education, CODEOWNERs, standard guides, refactors, and more. However, none of these addressed the root cause. 

## The Solution
With `tach`, you can:

1. Declare your modules ([`tach mod`](usage.md#tach-mod))
2. Define dependencies between modules ([`tach.yml`](configuration.md#tachyml))
3. Enforce those dependencies ([`tach check`](usage.md#tach-check))

You can also enforce a strict interface for each module. This means that only members that are listed in `__all__` (in the `__init__.py` of a package or in a standalone Python file) can be imported by other modules.

`tach` is:

- fully open source
- able to be adopted incrementally
- implemented with no runtime footprint
- interoperable with your existing tooling
