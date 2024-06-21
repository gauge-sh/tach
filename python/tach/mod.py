from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path
from typing import TYPE_CHECKING

from tach import errors
from tach import filesystem as fs
from tach.colors import BCOLORS
from tach.constants import CONFIG_FILE_NAME
from tach.interactive import (
    InteractiveModuleConfiguration,
    get_selected_modules_interactive,
)
from tach.parsing import dump_project_config_to_yaml

if TYPE_CHECKING:
    from tach.core import ProjectConfig


def update_modules(
    project_config: ProjectConfig,
    project_root: Path,
    selected_source_root: Path,
    selected_modules: list[Path],
):
    if project_config.source_root != selected_source_root:
        # Only assign to this field if it has changed,
        # since the project config writes any field that
        # has been touched out to YML.
        project_config.source_root = selected_source_root.relative_to(project_root)

    module_paths = [
        fs.file_to_module_path(
            source_root=selected_source_root, file_path=selected_module_file_path
        )
        for selected_module_file_path in selected_modules
    ]
    project_config.set_modules(module_paths=module_paths)

    project_config_path = project_root / f"{CONFIG_FILE_NAME}.yml"
    config_yml_content = dump_project_config_to_yaml(project_config)
    fs.write_file(str(project_config_path), config_yml_content)


@dataclass
class ValidationResult:
    ok: bool
    errors: list[str] = field(default_factory=list)


def validate_configuration(
    configuration: InteractiveModuleConfiguration,
) -> ValidationResult:
    errors: list[str] = []
    for module_path in configuration.module_paths:
        module_path = Path(module_path).resolve()

        if configuration.source_root not in module_path.parents:
            # This module exists outside of the source root
            # This is not allowed and should be reported as a configuration error
            errors.append(
                f"Module '{module_path}' is not contained within source root: '{configuration.source_root}'"
            )
    return ValidationResult(ok=not errors, errors=errors)


def mod_edit_interactive(
    project_root: Path, project_config: ProjectConfig, depth: int | None = 1
) -> tuple[bool, list[str]]:
    if not Path(project_root).is_dir():
        raise errors.TachSetupError(f"The path {project_root} is not a directory.")

    interactive_module_configuration = get_selected_modules_interactive(
        path=project_root,
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
            project_config=project_config,
            project_root=project_root,
            selected_source_root=interactive_module_configuration.source_root,
            selected_modules=interactive_module_configuration.module_paths,
        )
        return True, []
    else:
        return False, [f"{BCOLORS.OKCYAN}No changes saved.{BCOLORS.ENDC}"]


__all__ = ["mod_edit_interactive"]
