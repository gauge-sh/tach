from __future__ import annotations

from typing import TYPE_CHECKING

from tach import errors
from tach import filesystem as fs
from tach.check import check
from tach.filesystem import get_project_config_path
from tach.parsing import dump_project_config_to_yaml

if TYPE_CHECKING:
    from pathlib import Path

    from tach.core import ProjectConfig


def sync_dependency_constraints(
    project_root: Path,
    project_config: ProjectConfig,
    exclude_paths: list[str] | None = None,
) -> ProjectConfig:
    """
    Update project configuration with auto-detected dependency constraints.
    This is additive, meaning it will create dependencies to resolve existing errors,
    but will not remove any constraints.
    """
    check_result = check(
        project_root=project_root,
        project_config=project_config,
        exclude_paths=exclude_paths,
    )
    for error in check_result.errors:
        error_info = error.error_info
        if error_info.is_dependency_error:
            project_config.add_dependency_to_module(
                error_info.source_module, error_info.invalid_module
            )

    return project_config


def prune_dependency_constraints(
    project_root: Path,
    project_config: ProjectConfig,
    exclude_paths: list[str] | None = None,
) -> ProjectConfig:
    """
    Build a minimal project configuration with auto-detected module dependencies.
    """
    # Force module dependencies to be empty so that we can figure out the minimal set
    project_config = project_config.model_copy(
        update={
            "modules": [
                module.model_copy(update={"depends_on": []})
                for module in project_config.modules
            ]
        }
    )

    sync_dependency_constraints(
        project_root=project_root,
        project_config=project_config,
        exclude_paths=exclude_paths,
    )

    return project_config


def sync_project(
    project_root: Path,
    project_config: ProjectConfig,
    prune: bool = False,
    exclude_paths: list[str] | None = None,
) -> None:
    tach_yml_path = get_project_config_path(project_root)
    if tach_yml_path is None:
        raise errors.TachError(
            "Unexpected error. Could not find configuration file during 'sync'."
        )

    if prune:
        project_config = prune_dependency_constraints(
            project_root=project_root,
            project_config=project_config,
            exclude_paths=exclude_paths,
        )
    else:
        project_config = sync_dependency_constraints(
            project_root=project_root,
            project_config=project_config,
            exclude_paths=exclude_paths,
        )

    tach_yml_content = dump_project_config_to_yaml(project_config)
    fs.write_file(str(tach_yml_path), tach_yml_content)


__all__ = ["sync_project", "prune_dependency_constraints"]
