# Getting Started

## Installation

[PyPi package](https://pypi.org/project/tach/)


Install tach into a Python environment with `pip`

```bash
pip install tach
```

Verify your installation is working correctly
```bash
tach -h
```

## Adding to a Project

If you are adding `tach` to an existing project, you have two main options:

1. Use [`tach pkg`](usage.md#tach-pkg)  to interactively set up packages, and [`tach sync`](usage.md#tach-sync) to automatically set up dependency rules.
2. Manually configure your [packages](configuration.md#packageyml) and [dependency rules](configuration.md#tachyml)

## Checking Boundaries

```bash
# From the root of your Python project
tach check
```

After guarding your project, running `tach check` from the root will check all imports to verify that packages remain correctly decoupled.
