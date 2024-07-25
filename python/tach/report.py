from __future__ import annotations

from typing import TYPE_CHECKING

from tach import errors
from tach.extension import create_dependency_report, set_excluded_paths

if TYPE_CHECKING:
    from pathlib import Path

    from tach.core import ProjectConfig


def report(
    project_root: Path,
    path: Path,
    project_config: ProjectConfig,
    include_dependency_modules: list[str] | None = None,
    include_usage_modules: list[str] | None = None,
    skip_dependencies: bool = False,
    skip_usages: bool = False,
    exclude_paths: list[str] | None = None,
) -> str:
    if not project_root.is_dir():
        raise errors.TachSetupError(
            f"The path '{project_root}' is not a valid directory."
        )

    if not path.exists():
        raise errors.TachError(f"The path '{path}' does not exist.")

    if exclude_paths is not None and project_config.exclude is not None:
        exclude_paths.extend(project_config.exclude)
    else:
        exclude_paths = project_config.exclude

    # This informs the Rust extension ahead-of-time which paths are excluded.
    set_excluded_paths(exclude_paths=exclude_paths or [])

    return create_dependency_report(
        project_root=str(project_root),
        source_root=str(project_config.source_root),
        path=str(path),
        include_dependency_modules=include_dependency_modules,
        include_usage_modules=include_usage_modules,
        skip_dependencies=skip_dependencies,
        skip_usages=skip_usages,
        ignore_type_checking_imports=project_config.ignore_type_checking_imports,
    )


__all__ = ["report"]
