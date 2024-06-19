from __future__ import annotations

import os
from pathlib import Path

from tach import errors
from tach import filesystem as fs
from tach.colors import BCOLORS
from tach.constants import CONFIG_FILE_NAME
from tach.core import ProjectConfig
from tach.interactive import get_selected_modules_interactive
from tach.parsing import dump_project_config_to_yaml, parse_project_config


def update_modules(root: str, source_root: str, selected_modules: list[str]):
    project_config = parse_project_config(root=root) or ProjectConfig()

    module_paths = [
        fs.file_to_module_path(selected_module_file_path)
        for selected_module_file_path in selected_modules
    ]
    project_config.set_modules(module_paths=module_paths)

    project_config.source_root = fs.canonical(source_root)

    project_config_path = os.path.join(root, f"{CONFIG_FILE_NAME}.yml")
    config_yml_content = dump_project_config_to_yaml(project_config)
    fs.write_file(project_config_path, config_yml_content)


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
        update_modules(
            root=root,
            source_root=interactive_module_configuration.source_root,
            selected_modules=interactive_module_configuration.module_paths,
        )
        return True, []
    else:
        return False, [f"{BCOLORS.OKCYAN}No changes saved.{BCOLORS.ENDC}"]


__all__ = ["mod_edit_interactive"]
