from __future__ import annotations

import sys
from typing import Any

KNOWN_MODULE_SPECIAL_CASES = {
    "__future__",
    "typing_extensions",
}


def is_stdlib_module(module: str) -> bool:
    if module in KNOWN_MODULE_SPECIAL_CASES:
        return True

    if sys.version_info >= (3, 10):
        if module in sys.builtin_module_names:
            return True
        if module in sys.stdlib_module_names:
            return True
        return False
    else:
        from stdlib_list import in_stdlib  # type: ignore

        return in_stdlib(module)  # type: ignore


def get_installed_modules(dist: Any) -> list[str]:
    # This method is best-effort, and is only used for Python < 3.10
    module_names: set[str] = set()

    # Method 1: Check top_level.txt
    try:
        top_level = dist.read_text("top_level.txt")
        if top_level:
            module_names.update(top_level.strip().split())
    except Exception:
        pass

    # Method 2: Parse RECORD file
    try:
        record = dist.read_text("RECORD")
        if record:
            module_names.update(
                line.split(",")[0].split("/")[0]
                for line in record.splitlines()
                if "/" in line and not line.startswith("__")
            )
    except Exception:
        pass

    # Method 3: Check entry points
    for ep in dist.entry_points:
        if "." in ep.value:
            module_names.add(ep.value.split(":")[0])

    return list(module_names)


def get_module_mappings() -> dict[str, list[str]]:
    if sys.version_info >= (3, 10):
        from importlib.metadata import packages_distributions

        return packages_distributions()  # type: ignore
    else:
        if sys.version_info >= (3, 8):  # noqa: UP036
            from importlib.metadata import distributions
        else:
            from importlib_metadata import distributions  # type: ignore

        return {
            dist.metadata["Name"]: get_installed_modules(dist)
            for dist in distributions()
        }
