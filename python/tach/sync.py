from __future__ import annotations

from typing import TYPE_CHECKING

from tach import errors
from tach import filesystem as fs
from tach.check import check
from tach.core import Dependency
from tach.filesystem import get_project_config_path
from tach.parsing import dump_project_config_to_yaml

if TYPE_CHECKING:
    from pathlib import Path

    from tach.core import ProjectConfig


def sync_dependency_constraints(
    project_root: Path,
    project_config: ProjectConfig,
    exclude_paths: list[str] | None = None,
    prune: bool = True,
) -> ProjectConfig:
    """
    Update project configuration with auto-detected dependency constraints.
    If prune is set to False, it will create dependencies to resolve existing errors,
    but will not remove any constraints.
    """
    if prune:
        # Create a blank config
        new_config = project_config.model_copy(
            update={
                "modules": [
                    module.model_copy(update={"depends_on": []})
                    for module in project_config.modules
                ]
            }
        )
        # Update deprecations first
        check_result = check(
            project_root=project_root,
            project_config=project_config,
            exclude_paths=exclude_paths,
        )
        for warning in check_result.deprecated_warnings:
            new_config.add_dependency_to_module(
                warning.error_info.source_module,
                Dependency(path=warning.error_info.invalid_module, deprecated=True),
            )
    else:
        # Use the same config, existing deprecations will remain
        new_config = project_config
    check_result = check(
        project_root=project_root,
        project_config=new_config,
        exclude_paths=exclude_paths,
    )
    for error in check_result.errors:
        error_info = error.error_info
        if error_info.is_dependency_error:
            new_config.add_dependency_to_module(
                error_info.source_module, Dependency(path=error_info.invalid_module)
            )

    return new_config


def sync_project(
    project_root: Path,
    project_config: ProjectConfig,
    add: bool = False,
    exclude_paths: list[str] | None = None,
) -> None:
    tach_yml_path = get_project_config_path(project_root)
    if tach_yml_path is None:
        raise errors.TachError(
            "Unexpected error. Could not find configuration file during 'sync'."
        )

    project_config = sync_dependency_constraints(
        project_root=project_root,
        project_config=project_config,
        exclude_paths=exclude_paths,
        prune=not add,
    )
    tach_yml_content = dump_project_config_to_yaml(project_config)
    fs.write_file(str(tach_yml_path), tach_yml_content)


__all__ = ["sync_project", "sync_dependency_constraints"]
