from __future__ import annotations

from typing import TYPE_CHECKING

from tach import errors
from tach import filesystem as fs
from tach.check import check
from tach.core import Dependency
from tach.filesystem import get_project_config_path
from tach.parsing import dump_project_config_to_toml

if TYPE_CHECKING:
    from pathlib import Path

    from tach.core import ModuleConfig, ProjectConfig


def sync_dependency_constraints(
    project_root: Path,
    project_config: ProjectConfig,
    exclude_paths: list[str],
    prune: bool = True,
) -> ProjectConfig:
    """
    Update project configuration with auto-detected dependency constraints.
    If prune is set to False, it will create dependencies to resolve existing errors,
    but will not remove any constraints.
    """
    deprecation_map: dict[str, list[str]] = {}
    if prune:
        existing_modules: list[ModuleConfig] = []
        for module in project_config.modules:
            # Filter out modules that are not found in the source roots
            module_path = fs.module_to_pyfile_or_dir_path(
                tuple(
                    project_root / source_root
                    for source_root in project_config.source_roots
                ),
                module.path,
            )
            if module_path is not None:
                existing_modules.append(module)
            # Track deprecations so they can be restored while creating the new project config
            for dependency in module.depends_on:
                if dependency.deprecated:
                    if module.path not in deprecation_map:
                        deprecation_map[module.path] = [dependency.path]
                    else:
                        deprecation_map[module.path].append(dependency.path)

        # Create a blank config
        new_config = project_config.model_copy(
            update={
                "modules": [
                    module.model_copy(update={"depends_on": []})
                    for module in existing_modules
                ]
            }
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
            source_path = error_info.source_module
            dep_path = error_info.invalid_module
            deprecated = (
                source_path in deprecation_map
                and dep_path in deprecation_map[error_info.source_module]
            )
            if deprecated:
                dependency = Dependency(
                    path=error_info.invalid_module, deprecated=deprecated
                )
            else:
                dependency = Dependency(path=error_info.invalid_module)

            new_config.add_dependency_to_module(
                module=source_path, dependency=dependency
            )
    return new_config


def sync_project(
    project_root: Path,
    project_config: ProjectConfig,
    exclude_paths: list[str],
    add: bool = False,
) -> None:
    config_path = get_project_config_path(project_root)
    if config_path is None:
        raise errors.TachError(
            "Unexpected error. Could not find configuration file during 'sync'."
        )

    project_config = sync_dependency_constraints(
        project_root=project_root,
        project_config=project_config,
        exclude_paths=exclude_paths,
        prune=not add,
    )

    config_toml_content = dump_project_config_to_toml(project_config)
    fs.write_file(str(config_path), config_toml_content)


__all__ = ["sync_project", "sync_dependency_constraints"]
