# Getting Started

## Installation

[PyPi package](https://pypi.org/project/modguard/)


Install modguard into a Python environment with `pip`

```bash
pip install modguard
```

Verify your installation is working correctly
```bash
modguard -h
```

## Guarding a Project

If you are adding `modguard` to an existing project, you have two main options:

1. Use [`modguard init`](usage.md#modguard-init) to automatically set up modules and identify necessary dependencies
2. Manually configure your [modules](configuration.md#moduleyml) and [dependency rules](configuration.md#modguardyml)

## Checking Boundaries

```bash
# From the root of your Python project
modguard check
```

After guarding your project, running `modguard check` from the root will check all imports to verify that packages remain correctly decoupled.
