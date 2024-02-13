# Getting Started

## Installation

[PyPi package](https://pypi.org/project/modguard/)


Install modguard into a Python environment with `pip`

```bash
pip install modguard
```

Verify your installation is working correctly
```bash
modguard --help
```

## Guarding a Project

If you are adding `modguard` to an existing project, you have two main options:

1. Use [`modguard init`](usage.md#modguard-init) to automatically set up package boundaries and identify necessary public members
2. Manually mark boundaries and public members in your code ([See API docs](api.md))

## Checking Boundaries

```bash
# From the root of your Python project
modguard .
```

After guarding your project, running `modguard` from the root will check all imports to verify that packages remain correctly decoupled.
