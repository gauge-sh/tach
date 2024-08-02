from __future__ import annotations

import sys
from dataclasses import dataclass
from typing import TYPE_CHECKING

from tach.extension import check_external_dependencies, set_excluded_paths

if TYPE_CHECKING:
    from pathlib import Path

    from tach.core.config import ProjectConfig


def is_stdlib_module(module: str) -> bool:
    # Check for __future__
    if module == "__future__":
        return True

    if sys.version_info >= (3, 10):
        if module in sys.builtin_module_names:
            return True
        if module in sys.stdlib_module_names:
            return True
    else:
        from stdlib_list import in_stdlib  # type: ignore

        return in_stdlib(module)  # type: ignore


@dataclass
class ExternalCheckDiagnosticts:
    errors: list[str]


def check_external(
    project_root: Path, project_config: ProjectConfig
) -> ExternalCheckDiagnosticts:
    serialized_source_roots = [
        str(project_root / source_root) for source_root in project_config.source_roots
    ]
    if project_config.exclude:
        set_excluded_paths(
            project_root=str(project_root), exclude_paths=project_config.exclude
        )
    diagnostics = check_external_dependencies(
        project_root=str(project_root),
        source_roots=serialized_source_roots,
        ignore_type_checking_imports=project_config.ignore_type_checking_imports,
    )

    errors: list[str] = []
    for filepath, undeclared_dependencies in diagnostics.items():
        filtered_undeclared_dependencies = set(
            filter(
                lambda dependency: not is_stdlib_module(dependency),
                undeclared_dependencies,
            )
        )
        if filtered_undeclared_dependencies:
            errors.append(
                f"File '{filepath}' has undeclared dependencies: {', '.join(filtered_undeclared_dependencies)}"
            )

    return ExternalCheckDiagnosticts(errors=errors)
