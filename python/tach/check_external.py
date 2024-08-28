from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING

from tach.extension import check_external_dependencies, set_excluded_paths
from tach.utils.external import get_module_mappings, is_stdlib_module

if TYPE_CHECKING:
    from pathlib import Path

    from tach.core.config import ProjectConfig


@dataclass
class ExternalCheckDiagnosticts:
    undeclared_dependencies: dict[str, list[str]]


def check_external(
    project_root: Path,
    project_config: ProjectConfig,
    exclude_paths: list[str],
) -> ExternalCheckDiagnosticts:
    serialized_source_roots = [
        str(project_root / source_root) for source_root in project_config.source_roots
    ]
    set_excluded_paths(
        project_root=str(project_root),
        exclude_paths=exclude_paths,
        use_regex_matching=project_config.use_regex_matching,
    )

    diagnostics = check_external_dependencies(
        project_root=str(project_root),
        source_roots=serialized_source_roots,
        module_mappings=get_module_mappings(),
        ignore_type_checking_imports=project_config.ignore_type_checking_imports,
    )

    excluded_external_modules = set(project_config.external.exclude)
    all_undeclared_dependencies: dict[str, list[str]] = {}
    for filepath, undeclared_dependencies in diagnostics.items():
        filtered_undeclared_dependencies = set(
            filter(
                lambda dependency: not is_stdlib_module(dependency)
                and dependency not in excluded_external_modules,
                undeclared_dependencies,
            )
        )
        if filtered_undeclared_dependencies:
            all_undeclared_dependencies[filepath] = list(
                filtered_undeclared_dependencies
            )

    return ExternalCheckDiagnosticts(
        undeclared_dependencies=all_undeclared_dependencies
    )
