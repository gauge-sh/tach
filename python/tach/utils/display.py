from __future__ import annotations

import os
import sys
from enum import Enum
from functools import lru_cache
from pathlib import Path
from typing import TYPE_CHECKING

from tach.colors import BCOLORS
from tach.constants import CONFIG_FILE_NAME, TOOL_NAME

if TYPE_CHECKING:
    from pathlib import Path

    from tach.check import BoundaryError
    from tach.core import UnusedDependencies


class TerminalEnvironment(Enum):
    UNKNOWN = 1
    JETBRAINS = 2
    VSCODE = 3


@lru_cache(maxsize=None)
def detect_environment() -> TerminalEnvironment:
    if "jetbrains" in os.environ.get("TERMINAL_EMULATOR", "").lower():
        return TerminalEnvironment.JETBRAINS
    elif "vscode" in os.environ.get("TERM_PROGRAM", "").lower():
        return TerminalEnvironment.VSCODE
    return TerminalEnvironment.UNKNOWN


def create_clickable_link(
    file_path: Path, display_path: Path | None = None, line: int | None = None
) -> str:
    terminal_env = detect_environment()
    abs_path = file_path.resolve()

    if terminal_env == TerminalEnvironment.JETBRAINS:
        link = f"file://{abs_path}:{line}" if line is not None else f"file://{abs_path}"
    elif terminal_env == TerminalEnvironment.VSCODE:
        link = (
            f"vscode://file/{abs_path}:{line}"
            if line is not None
            else f"vscode://file/{abs_path}"
        )
    else:
        # For generic terminals, use a standard file link
        link = f"file://{abs_path}"

    # ANSI escape codes for clickable link
    if line:
        # Show the line number if we have it
        display_file_path = f"{display_path or file_path}[L{line}]"
    else:
        display_file_path = str(display_path) if display_path else str(file_path)
    clickable_link = f"\033]8;;{link}\033\\{display_file_path}\033]8;;\033\\"
    return clickable_link


def build_absolute_error_path(file_path: Path, source_roots: list[Path]) -> Path:
    absolute_error_path = next(
        (
            source_root / file_path
            for source_root in source_roots
            if (source_root / file_path).exists()
        ),
        None,
    )

    if absolute_error_path is None:
        # This is an unexpected case,
        # all errors should have originated from within a source root
        return file_path
    return absolute_error_path


def build_error_message(error: BoundaryError, source_roots: list[Path]) -> str:
    absolute_error_path = build_absolute_error_path(
        file_path=error.file_path, source_roots=source_roots
    )

    if absolute_error_path == error.file_path:
        # This is an unexpected case,
        # all errors should have originated from within a source root
        error_location = error.file_path
    else:
        error_location = create_clickable_link(
            absolute_error_path,
            display_path=error.file_path,
            line=error.line_number,
        )

    error_template = (
        f"❌ {BCOLORS.FAIL}{error_location}{BCOLORS.ENDC}{BCOLORS.FAIL}: "
        f"{{message}} {BCOLORS.ENDC}"
    )
    warning_template = (
        f"‼️ {BCOLORS.FAIL}{error_location}{BCOLORS.ENDC}{BCOLORS.WARNING}: "
        f"{{message}} {BCOLORS.ENDC}"
    )
    error_info = error.error_info
    if error_info.exception_message:
        return error_template.format(message=error_info.exception_message)
    elif not error_info.is_dependency_error:
        return error_template.format(message="Unexpected error")

    error_message = (
        f"Cannot import '{error.import_mod_path}'. "
        f"Module '{error_info.source_module}' cannot depend on '{error_info.invalid_module}'."
    )

    warning_message = (
        f"Import '{error.import_mod_path}' is deprecated. "
        f"Module '{error_info.source_module}' should not depend on '{error_info.invalid_module}'."
    )
    if error_info.is_deprecated:
        return warning_template.format(message=warning_message)
    return error_template.format(message=error_message)


def print_warnings(warning_list: list[str]) -> None:
    for warning in warning_list:
        print(f"{BCOLORS.WARNING}{warning}{BCOLORS.ENDC}", file=sys.stderr)


def print_errors(error_list: list[BoundaryError], source_roots: list[Path]) -> None:
    if not error_list:
        return
    sorted_results = sorted(error_list, key=lambda e: e.file_path)
    for error in sorted_results:
        print(
            build_error_message(error, source_roots=source_roots),
            file=sys.stderr,
        )
    if not all(error.error_info.is_deprecated for error in sorted_results):
        print(
            f"{BCOLORS.WARNING}\nIf you intended to add a new dependency, run 'tach sync' to update your module configuration."
            f"\nOtherwise, remove any disallowed imports and consider refactoring.\n{BCOLORS.ENDC}"
        )


def print_unused_dependencies(
    all_unused_dependencies: list[UnusedDependencies],
) -> None:
    constraint_messages = "\n".join(
        f"\t{BCOLORS.WARNING}'{unused_dependencies.path}' does not depend on: {[dependency.path for dependency in unused_dependencies.dependencies]}{BCOLORS.ENDC}"
        for unused_dependencies in all_unused_dependencies
    )
    print(
        f"❌ {BCOLORS.FAIL}Found unused dependencies: {BCOLORS.ENDC}\n"
        + constraint_messages
    )
    print(
        f"{BCOLORS.WARNING}\nRemove the unused dependencies from {CONFIG_FILE_NAME}.toml, "
        f"or consider running '{TOOL_NAME} sync' to update module configuration and "
        f"remove all unused dependencies.\n{BCOLORS.ENDC}"
    )


def print_no_config_found() -> None:
    print(
        f"{BCOLORS.FAIL} {CONFIG_FILE_NAME}.toml not found{BCOLORS.ENDC}",
        file=sys.stderr,
    )


def print_show_web_suggestion() -> None:
    print(
        f"{BCOLORS.OKCYAN}NOTE: You are generating a DOT file locally representing your module graph. For a remotely hosted visualization, use the '--web' argument.\nTo visualize your graph, you will need a program like GraphViz: https://www.graphviz.org/download/\n{BCOLORS.ENDC}"
    )


def print_generated_module_graph_file(output_filepath: Path) -> None:
    print(
        f"{BCOLORS.OKGREEN}Generated a DOT file containing your module graph at '{output_filepath}'{BCOLORS.ENDC}"
    )


def print_circular_dependency_error(module_paths: list[str]) -> None:
    print(
        "\n".join(
            [
                f"❌ {BCOLORS.FAIL}Circular dependency detected for module {BCOLORS.ENDC}'{module_path}'"
                for module_path in module_paths
            ]
        )
        + f"\n\n{BCOLORS.WARNING}Resolve circular dependencies.\n"
        f"Remove or unset 'forbid_circular_dependencies' from "
        f"'{CONFIG_FILE_NAME}.toml' to allow circular dependencies.{BCOLORS.ENDC}"
    )


def print_undeclared_dependencies(
    undeclared_dependencies: dict[str, list[str]],
) -> None:
    for file_path, dependencies in undeclared_dependencies.items():
        print(
            f"❌ {BCOLORS.FAIL}Undeclared dependencies in {BCOLORS.ENDC}{BCOLORS.WARNING}'{file_path}'{BCOLORS.ENDC}:"
        )
        for dependency in dependencies:
            print(f"\t{BCOLORS.FAIL}{dependency}{BCOLORS.ENDC}")
    print(
        f"{BCOLORS.WARNING}\nAdd the undeclared dependencies to the corresponding pyproject.toml file, "
        f"or consider ignoring the dependencies by adding them to the 'external.exclude' list in {CONFIG_FILE_NAME}.toml.\n{BCOLORS.ENDC}"
    )
