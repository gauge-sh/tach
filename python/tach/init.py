from __future__ import annotations

from typing import TYPE_CHECKING

from tach import errors
from tach import filesystem as fs
from tach.constants import CONFIG_FILE_NAME, TOOL_NAME
from tach.extension import ProjectConfig, parse_project_config, sync_project
from tach.mod import mod_edit_interactive
from tach.show import generate_show_url

if TYPE_CHECKING:
    from pathlib import Path


def mark_modules(project_root: Path, project_config: ProjectConfig) -> ProjectConfig:
    saved, _ = mod_edit_interactive(
        project_root=project_root,
        project_config=project_config,
        exclude_paths=project_config.exclude,
    )
    if not saved:
        raise errors.TachInitCancelledError()

    project_config_path = fs.build_project_config_path(project_root)
    project_config, _ = parse_project_config(project_config_path)
    return project_config


def sync_modules(project_root: Path, project_config: ProjectConfig) -> ProjectConfig:
    sync_project(project_root, project_config)

    project_config_path = fs.build_project_config_path(project_root)
    project_config, _ = parse_project_config(project_config_path)
    return project_config


def prompt_to_re_select_modules() -> bool:
    print(
        "No dependencies found between the selected modules."
        + " This might mean that your source root is not set correctly,"
        + " or you did not select modules which depend on each other."
    )
    return input("Would you like to re-select modules?" + "\n  (y/n):  ").lower() == "y"


def setup_modules(project_root: Path, project_config: ProjectConfig) -> ProjectConfig:
    project_config = mark_modules(project_root, project_config)
    project_config = sync_modules(project_root, project_config)

    while project_config.has_no_dependencies():
        wants_to_re_select = prompt_to_re_select_modules()
        if wants_to_re_select:
            project_config = mark_modules(project_root, project_config)
            project_config = sync_modules(project_root, project_config)
        else:
            print("Continuing with selected modules.")
            break
    return project_config


def prompt_to_show_project() -> bool:
    return (
        input(
            "Would you like to visualize your dependency graph?"
            + f"\nThis will upload your module configuration from '{CONFIG_FILE_NAME}.toml' to Gauge [https://app.gauge.sh/show]."
            + "\n  (y/n):  "
        ).lower()
        == "y"
    )


def show_project(project_config: ProjectConfig):
    if prompt_to_show_project():
        show_url = generate_show_url(project_config)
        if show_url:
            print("View your dependency graph here:")
            print(show_url)
        else:
            print("Failed to generate show URL. Please try again later.")


def print_intro():
    print(
        "Tach is now configured for this project."
        + " You can now run `tach check` to validate your configuration."
    )


def prompt_to_reinitialize_project() -> bool:
    return (
        input(
            "Project already initialized. Would you like to reinitialize?"
            + " This will overwrite the current configuration."
            + "\n  (y/n):  "
        ).lower()
        == "y"
    )


def init_project(project_root: Path, force: bool = False):
    current_config_path = fs.get_project_config_path(project_root)
    if current_config_path:
        if not force:
            raise errors.TachError(
                f"Project already initialized. Use `{TOOL_NAME} init --force` to reinitialize."
            )
        else:
            if prompt_to_reinitialize_project():
                try:
                    current_config_path.unlink()
                    for domain_file in project_root.rglob("tach.domain.toml"):
                        domain_file.unlink()
                except OSError:
                    raise errors.TachError("Failed to remove configuration file.")
            else:
                raise errors.TachError(
                    "Refusing to overwrite existing project configuration."
                )

    project_config = ProjectConfig()

    try:
        project_config = setup_modules(project_root, project_config)
    except errors.TachInitCancelledError:
        return
    show_project(project_config)
    print_intro()
