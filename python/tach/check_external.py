from __future__ import annotations

from typing import TYPE_CHECKING

from tach.errors import TachError
from tach.extension import (
    ExternalCheckDiagnostics,
    check_external_dependencies,
    set_excluded_paths,
)
from tach.utils.external import (
    get_module_mappings,
    get_stdlib_modules,
)

if TYPE_CHECKING:
    from pathlib import Path

    from tach.extension import ProjectConfig


def extract_module_mappings(rename: list[str]) -> dict[str, list[str]]:
    try:
        return {
            module: [name] for module, name in [module.split(":") for module in rename]
        }
    except ValueError as e:
        raise TachError(
            "Invalid rename format: expected format is a list of 'module:name' pairs, e.g. ['PIL:pillow']"
        ) from e


def check_external(
    project_root: Path,
    project_config: ProjectConfig,
    exclude_paths: list[str],
) -> ExternalCheckDiagnostics:
    set_excluded_paths(
        project_root=str(project_root),
        exclude_paths=exclude_paths,
        use_regex_matching=project_config.use_regex_matching,
    )

    metadata_module_mappings = get_module_mappings()
    if project_config.external.rename:
        metadata_module_mappings.update(
            extract_module_mappings(project_config.external.rename)
        )
    return check_external_dependencies(
        project_root=str(project_root),
        project_config=project_config,
        module_mappings=metadata_module_mappings,
        stdlib_modules=get_stdlib_modules(),
    )


__all__ = ["check_external"]
