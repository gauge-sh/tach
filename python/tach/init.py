from __future__ import annotations

from typing import TYPE_CHECKING

from rich.console import Console
from rich.panel import Panel
from rich.prompt import Confirm

from tach import errors
from tach import filesystem as fs
from tach.constants import CONFIG_FILE_NAME, TOOL_NAME
from tach.extension import ProjectConfig, parse_project_config, sync_project
from tach.mod import mod_edit_interactive
from tach.show import generate_show_url

if TYPE_CHECKING:
    from pathlib import Path

console = Console()


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
    console.print(
        Panel(
            "No dependencies found between the selected modules.\n"
            "This might mean that your source root is not set correctly,"
            " or you did not select modules which depend on each other.",
            style="yellow",
        )
    )
    return Confirm.ask(
        "[cyan]Would you like to re-select modules?[/]\n",
        default=True,
        show_default=False,
    )


def prompt_to_show_project() -> bool:
    console.print(
        Panel(
            "Would you like to visualize your dependency graph?\n"
            f"This will upload your module configuration in [cyan]'{CONFIG_FILE_NAME}.toml'[/] to Gauge ([blue underline]https://app.gauge.sh[/])",
            style="yellow",
        )
    )
    return Confirm.ask("", default=False, show_default=False)


def show_project(project_config: ProjectConfig):
    if prompt_to_show_project():
        show_url = generate_show_url(project_config)
        if show_url:
            console.print(
                "[green]View your dependency graph here:[/]\n"
                f"[blue underline]{show_url}[/]"
            )
        else:
            console.print(
                "[red]Failed to generate show URL. Please try again later.[/]"
            )


def setup_modules(project_root: Path, project_config: ProjectConfig) -> ProjectConfig:
    project_config = mark_modules(project_root, project_config)
    project_config = sync_modules(project_root, project_config)

    while project_config.has_no_dependencies():
        wants_to_re_select = prompt_to_re_select_modules()
        if wants_to_re_select:
            project_config = mark_modules(project_root, project_config)
            project_config = sync_modules(project_root, project_config)
        else:
            console.print("[cyan]Continuing with selected modules.[/]")
            break
    return project_config


def init_project(project_root: Path, force: bool = False):
    current_config_path = fs.get_project_config_path(project_root)
    if current_config_path:
        if not force:
            raise errors.TachError(
                f"[yellow]Project already initialized. Use [cyan]`{TOOL_NAME} init --force`[/] to reinitialize.[/]"
            )

        console.print(
            Panel(
                "Project already initialized. Would you like to reinitialize?\n"
                "This will overwrite the current configuration.",
                style="yellow",
            )
        )

        if Confirm.ask("", default=False, show_default=False):
            try:
                current_config_path.unlink()
                for domain_file in project_root.rglob("tach.domain.toml"):
                    domain_file.unlink()
            except OSError:
                raise errors.TachError("[red]Failed to remove configuration file.[/]")
        else:
            raise errors.TachError(
                "[red]Refusing to overwrite existing project configuration.[/]"
            )

    project_config = ProjectConfig()

    try:
        project_config = setup_modules(project_root, project_config)
    except errors.TachInitCancelledError:
        return

    show_project(project_config)

    console.print(
        "[yellow]Tach is now configured for this project!"
        " You can run [cyan]'tach check'[/] to validate your configuration.[/]\n"
        "[yellow]Documentation is available at [blue underline]https://docs.gauge.sh[/]"
    )
