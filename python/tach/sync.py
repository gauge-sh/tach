from __future__ import annotations

import os
from typing import TYPE_CHECKING

from tach import errors
from tach import filesystem as fs
from tach.check import check
from tach.colors import BCOLORS
from tach.constants import CONFIG_FILE_NAME
from tach.parsing import dump_project_config_to_yaml, parse_project_config

if TYPE_CHECKING:
    from tach.core import ProjectConfig


def sync_dependency_constraints(
    root: str, project_config: ProjectConfig, exclude_paths: list[str] | None = None
) -> ProjectConfig:
    """
    Update project configuration with auto-detected dependency constraints.
    This is additive, meaning it will create dependencies to resolve existing errors,
    but will not remove any constraints.
    """
    check_result = check(
        root, project_config=project_config, exclude_paths=exclude_paths
    )
    for error in check_result.errors:
        error_info = error.error_info
        if error_info.is_dependency_error:
            project_config.add_dependency_to_module(
                error_info.source_module, error_info.invalid_module
            )

    return project_config


def prune_dependency_constraints(
    root: str,
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
        root, project_config=project_config, exclude_paths=exclude_paths
    )

    return project_config


def sync_project(prune: bool = False, exclude_paths: list[str] | None = None) -> None:
    original_cwd = fs.get_cwd()
    try:
        root = fs.find_project_config_root(original_cwd)
        if root is None:
            raise errors.TachSetupError(
                f"{BCOLORS.WARNING}Could not find project configuration.{BCOLORS.ENDC}"
            )
        fs.chdir(root)
        project_config = parse_project_config(root=root)
        if not project_config:
            raise errors.TachSetupError(
                f"{BCOLORS.WARNING}Could not find project configuration.{BCOLORS.ENDC}"
            )

        if exclude_paths is not None and project_config.exclude is not None:
            exclude_paths.extend(project_config.exclude)
        else:
            exclude_paths = project_config.exclude

        if prune:
            project_config = prune_dependency_constraints(
                root, project_config=project_config, exclude_paths=exclude_paths
            )
        else:
            project_config = sync_dependency_constraints(
                root, project_config=project_config, exclude_paths=exclude_paths
            )

        tach_yml_path = os.path.join(root, f"{CONFIG_FILE_NAME}.yml")
        tach_yml_content = dump_project_config_to_yaml(project_config)
        fs.write_file(tach_yml_path, tach_yml_content)

    finally:
        fs.chdir(original_cwd)


__all__ = ["sync_project", "prune_dependency_constraints"]
