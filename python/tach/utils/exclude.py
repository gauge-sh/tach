from __future__ import annotations

import fnmatch
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Generator
    from pathlib import Path


def _with_optional_trailing_slashes(patterns: list[str]) -> Generator[str, None, None]:
    for pattern in patterns:
        yield pattern
        if pattern.endswith("/"):
            yield pattern[:-1]


# Assumes 'relative_path' is a path relative to the project root
def is_path_excluded(exclude_paths: list[str], relative_path: Path) -> bool:
    if not exclude_paths:
        return False

    return any(
        (fnmatch.fnmatch(str(relative_path), exclude_path))
        for exclude_path in _with_optional_trailing_slashes(exclude_paths)
    )
