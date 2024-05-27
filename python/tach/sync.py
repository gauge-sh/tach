from __future__ import annotations

import os
from typing import Optional

from tach import errors
from tach import filesystem as fs
from tach.check import check
from tach.colors import BCOLORS
from tach.constants import CONFIG_FILE_NAME
from tach.core import ProjectConfig
from tach.parsing import dump_project_config_to_yaml, parse_project_config


def sync_dependency_constraints(
    root: str,
    project_config: ProjectConfig,
    filter_tags: Optional[set[str]] = None,
    exclude_paths: Optional[list[str]] = None,
) -> ProjectConfig:
    """
    Update project configuration with auto-detected dependency constraints.
    This is additive, meaning it will create dependencies to resolve existing errors,
    but will not remove any constraints.
    Passing 'filter_tags' will limit updates to only those dependencies which include one of those tags.
    """
    check_errors = check(
        root, project_config=project_config, exclude_paths=exclude_paths
    )
    for error in check_errors:
        error_info = error.error_info
        if error_info.is_tag_error:
            if not filter_tags:
                project_config.add_dependencies_to_tags(
                    error_info.source_tags, error_info.invalid_tags
                )
            else:
                source_tags = set(error_info.source_tags)
                invalid_tags = set(error_info.invalid_tags)
                if source_tags & filter_tags:
                    # A package with one of the added tags caused this error and should update its dependencies
                    project_config.add_dependencies_to_tags(
                        error_info.source_tags, error_info.invalid_tags
                    )
                if invalid_tags & filter_tags:
                    # A package now depends on one of the added tags and should add the newly added tags
                    # Note that we should leave pre-existing invalid tags
                    project_config.add_dependencies_to_tags(
                        error_info.source_tags, list(invalid_tags & filter_tags)
                    )

    return project_config


def prune_dependency_constraints(
    root: str,
    project_config: Optional[ProjectConfig] = None,
    exclude_paths: Optional[list[str]] = None,
) -> ProjectConfig:
    """
    Build a minimal project configuration with auto-detected dependency constraints.
    """
    if project_config is not None:
        # Force constraints to be empty in case we received configuration with pre-existing constraints
        project_config = project_config.model_copy(update={"constraints": []})
    else:
        project_config = ProjectConfig()

    sync_dependency_constraints(
        root, project_config=project_config, exclude_paths=exclude_paths
    )

    return project_config


def sync_project(
    prune: bool = False, exclude_paths: Optional[list[str]] = None
) -> None:
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
