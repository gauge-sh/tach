from __future__ import annotations

import os
from dataclasses import dataclass, field
from pathlib import Path

from tach import errors
from tach import filesystem as fs
from tach.colors import BCOLORS
from tach.constants import CONFIG_FILE_NAME
from tach.core import ProjectConfig
from tach.interactive import (
    InteractiveModuleConfiguration,
    get_selected_modules_interactive,
)
from tach.parsing import dump_project_config_to_yaml, parse_project_config


def update_modules(root: str, source_root: str, selected_modules: list[str]):
    project_config = parse_project_config(root=root) or ProjectConfig()

    module_paths = [
        fs.file_to_module_path(selected_module_file_path)
        for selected_module_file_path in selected_modules
    ]
    project_config.set_modules(module_paths=module_paths)

    source_root = fs.canonical(source_root)
    if project_config.source_root != source_root:
        # Only assign to this field if it has changed,
        # since the project config writes any field that
        # has been touched out to YML.
        project_config.source_root = fs.canonical(source_root)

    project_config_path = os.path.join(root, f"{CONFIG_FILE_NAME}.yml")
    config_yml_content = dump_project_config_to_yaml(project_config)
    fs.write_file(project_config_path, config_yml_content)


@dataclass
class ValidationResult:
    ok: bool
    errors: list[str] = field(default_factory=list)


def validate_configuration(
    configuration: InteractiveModuleConfiguration,
) -> ValidationResult:
    source_root = Path(configuration.source_root).resolve()
    errors: list[str] = []
    for module_path in configuration.module_paths:
        module_path = Path(module_path).resolve()

        if source_root not in module_path.parents:
            # This module exists outside of the source root
            # This is not allowed and should be reported as a configuration error
            errors.append(
                f"Module '{fs.canonical(str(module_path))}' is not contained within source root: '{fs.canonical(str(source_root))}'"
            )
    return ValidationResult(ok=not errors, errors=errors)


def mod_edit_interactive(
    root: str, project_config: ProjectConfig, depth: int | None = 1
) -> tuple[bool, list[str]]:
    if not Path(root).is_dir():
        raise errors.TachSetupError(f"The path {root} is not a directory.")

    interactive_module_configuration = get_selected_modules_interactive(
        root,
        project_config=project_config,
        depth=depth,
    )
    if interactive_module_configuration is not None:
        validation_result = validate_configuration(interactive_module_configuration)
        if validation_result.errors:
            return False, [
                f"{BCOLORS.FAIL}Validation error: {BCOLORS.WARNING}{error}{BCOLORS.ENDC}"
                for error in validation_result.errors
            ]
        update_modules(
            root=root,
            source_root=interactive_module_configuration.source_root,
            selected_modules=interactive_module_configuration.module_paths,
        )
        return True, []
    else:
        return False, [f"{BCOLORS.OKCYAN}No changes saved.{BCOLORS.ENDC}"]


__all__ = ["mod_edit_interactive"]
