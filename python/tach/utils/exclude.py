from __future__ import annotations

import fnmatch
import re
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from pathlib import Path


# Assumes 'relative_path' is a path relative to the project root
def is_path_excluded(
    exclude_paths: list[str], relative_path: Path, use_regex_matching: bool
) -> bool:
    if not exclude_paths:
        return False

    # TODO: deprecate regex excludes
    path_for_regex = f"{relative_path}/"
    return any(
        (
            re.match(exclude_path, path_for_regex)
            if use_regex_matching
            else fnmatch.fnmatch(str(relative_path), exclude_path)
        )
        for exclude_path in exclude_paths
    )
