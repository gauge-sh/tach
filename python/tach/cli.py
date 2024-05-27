from __future__ import annotations

import argparse
import os
import sys
from enum import Enum
from functools import lru_cache
from pathlib import Path
from typing import TYPE_CHECKING, Optional

from tach import filesystem as fs
from tach.check import BoundaryError, check
from tach.clean import clean_project
from tach.colors import BCOLORS
from tach.constants import CONFIG_FILE_NAME, TOOL_NAME
from tach.filesystem import install_pre_commit
from tach.loading import start_spinner, stop_spinner
from tach.logging import LogDataModel, logger
from tach.parsing import parse_project_config
from tach.pkg import pkg_edit_interactive
from tach.sync import prune_dependency_constraints, sync_project

if TYPE_CHECKING:
    from tach.core import TagDependencyRules


class TerminalEnvironment(Enum):
    UNKNOWN = 1
    JETBRAINS = 2
    VSCODE = 3


@lru_cache()
def detect_environment() -> TerminalEnvironment:
    if "jetbrains" in os.environ.get("TERMINAL_EMULATOR", "").lower():
        return TerminalEnvironment.JETBRAINS
    elif "vscode" in os.environ.get("TERM_PROGRAM", "").lower():
        return TerminalEnvironment.VSCODE
    return TerminalEnvironment.UNKNOWN


def create_clickable_link(file_path: str, line: Optional[int] = None) -> str:
    terminal_env = detect_environment()
    abs_path = os.path.abspath(file_path)

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
    if line and terminal_env != TerminalEnvironment.UNKNOWN:
        # Show the line number if clicking will take you to the line
        display_file_path = f"{file_path}[L{line}]"
    else:
        display_file_path = file_path
    clickable_link = f"\033]8;;{link}\033\\{display_file_path}\033]8;;\033\\"
    return clickable_link


def build_error_message(error: BoundaryError) -> str:
    error_location = create_clickable_link(error.file_path, error.line_number)
    error_template = f"❌ {BCOLORS.FAIL}{error_location}{BCOLORS.ENDC}{BCOLORS.WARNING}: {{message}} {BCOLORS.ENDC}"
    error_info = error.error_info
    if error_info.exception_message:
        return error_template.format(message=error_info.exception_message)
    elif not error_info.is_tag_error:
        return error_template.format(message="Unexpected error")

    message = (
        f"Cannot import '{error.import_mod_path}'. "
        f"Tags {error_info.source_tags} cannot depend on {error_info.invalid_tags}."
    )

    return error_template.format(message=message)


def print_errors(error_list: list[BoundaryError]) -> None:
    sorted_results = sorted(error_list, key=lambda e: e.file_path)
    for error in sorted_results:
        print(
            build_error_message(error),
            file=sys.stderr,
        )


def print_extra_constraints(constraints: list[TagDependencyRules]) -> None:
    constraint_messages = "\n".join(
        f"\t{BCOLORS.WARNING}{constraint.tag} does not depend on: {constraint.depends_on}{BCOLORS.ENDC}"
        for constraint in constraints
    )
    print(
        f"❌ {BCOLORS.FAIL}Found unused dependencies: {BCOLORS.ENDC}\n"
        + constraint_messages
    )


def print_no_config_yml() -> None:
    print(
        f"{BCOLORS.FAIL} {CONFIG_FILE_NAME}.(yml|yaml) not found in {Path.cwd()}{BCOLORS.ENDC}",
        file=sys.stderr,
    )


def add_base_arguments(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "-e",
        "--exclude",
        required=False,
        type=str,
        metavar="file_or_path,...",
        help="Comma separated path list to exclude. tests/, ci/, etc.",
    )


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="tach",
        add_help=True,
        epilog="Make sure tach is run from the root of your Python project,"
        " and `tach.yml` is present",
    )
    subparsers = parser.add_subparsers(title="commands", dest="command")
    pkg_parser = subparsers.add_parser(
        "pkg",
        prog="tach pkg",
        help="Configure package boundaries interactively",
        description="Configure package boundaries interactively",
    )
    pkg_parser.add_argument(
        "-d",
        "--depth",
        type=int,
        nargs="?",
        default=None,
        help="The number of child directories to search for packages to auto-select",
    )
    add_base_arguments(pkg_parser)
    check_parser = subparsers.add_parser(
        "check",
        prog="tach check",
        help="Check existing boundaries against your dependencies and package interfaces",
        description="Check existing boundaries against your dependencies and package interfaces",
    )
    check_parser.add_argument(
        "--root",
        required=False,
        type=str,
        default=".",
        help="The root directory from which the check should run",
    )
    check_parser.add_argument(
        "--exact",
        action="store_true",
        help="Raise errors if any dependency constraints are unused.",
    )
    add_base_arguments(check_parser)
    install_parser = subparsers.add_parser(
        "install",
        prog="tach install",
        help="Install tach into your workflow (e.g. as a pre-commit hook)",
        description="Install tach into your workflow (e.g. as a pre-commit hook)",
    )
    install_parser.add_argument(
        "target",
        choices=InstallTarget.choices(),
        help="What kind of installation to perform (e.g. pre-commit)",
    )
    install_parser.add_argument(
        "-p",
        "--path",
        required=False,
        type=str,
        default=".",
        help="The path where this installation should occur (e.g. git root for hooks)",
    )
    install_parser.add_argument(
        "--project-root",
        required=False,
        type=str,
        default="",
        help="The relative path where 'tach check' should run (defaults to git root)",
    )
    sync_parser = subparsers.add_parser(
        "sync",
        prog="tach sync",
        help="Sync constraints with actual dependencies in your project.",
        description="Sync constraints with actual dependencies in your project.",
    )
    sync_parser.add_argument(
        "--prune",
        action="store_true",
        help="Prune all existing constraints and re-sync dependencies.",
    )
    add_base_arguments(sync_parser)
    clean_parser = subparsers.add_parser(
        "clean",
        prog="tach clean",
        help="Delete existing configuration and start from an empty slate.",
        description="Delete existing configuration and start from an empty slate.",
    )
    clean_parser.add_argument(
        "--force", action="store_true", help="Do not prompt for confirmation."
    )
    return parser


def parse_arguments(
    args: list[str],
) -> tuple[argparse.Namespace, argparse.ArgumentParser]:
    parser = build_parser()
    parsed_args = parser.parse_args(args)
    return parsed_args, parser


def tach_check(
    root: str = ".",
    exact: bool = False,
    exclude_paths: Optional[list[str]] = None,
):
    logger.info(
        "tach check called",
        extra={
            "data": LogDataModel(
                function="tach_check",
                parameters={"exact": exact},
            ),
        },
    )
    try:
        project_config = parse_project_config(root=root)
        if project_config is None:
            stop_spinner()
            print_no_config_yml()
            sys.exit(1)

        if exact is False and project_config.exact is True:
            exact = True

        if exclude_paths is not None and project_config.exclude is not None:
            exclude_paths.extend(project_config.exclude)
        else:
            exclude_paths = project_config.exclude

        boundary_errors: list[BoundaryError] = check(
            root,
            project_config,
            exclude_paths=exclude_paths,
        )

        # If we're checking in strict mode, we want to verify that pruning constraints has no effect
        if not boundary_errors and exact:
            pruned_config = prune_dependency_constraints(
                root, project_config=project_config, exclude_paths=exclude_paths
            )
            extra_constraints = pruned_config.find_extra_constraints(project_config)
            if extra_constraints:
                stop_spinner()
                print_extra_constraints(extra_constraints)
                sys.exit(1)
    except Exception as e:
        stop_spinner()
        print(str(e))
        sys.exit(1)

    stop_spinner()
    if boundary_errors:
        print_errors(boundary_errors)
        sys.exit(1)
    print(f"✅ {BCOLORS.OKGREEN}All package dependencies validated!{BCOLORS.ENDC}")
    sys.exit(0)


def tach_pkg(depth: Optional[int] = 1, exclude_paths: Optional[list[str]] = None):
    logger.info(
        "tach pkg called",
        extra={
            "data": LogDataModel(
                function="tach_pkg",
                parameters={"depth": depth},
            ),
        },
    )
    try:
        saved_changes, warnings = pkg_edit_interactive(
            root=".", depth=depth, exclude_paths=exclude_paths
        )
    except Exception as e:
        print(str(e))
        sys.exit(1)

    if warnings:
        print("\n".join(warnings))
    if saved_changes:
        print(
            f"✅ {BCOLORS.OKGREEN}Set packages! You may want to run '{TOOL_NAME} sync' "
            f"to automatically set boundaries.{BCOLORS.ENDC}"
        )
    sys.exit(0)


def tach_sync(prune: bool = False, exclude_paths: Optional[list[str]] = None):
    logger.info(
        "tach sync called",
        extra={
            "data": LogDataModel(
                function="tach_sync",
                parameters={"prune": prune},
            ),
        },
    )
    try:
        sync_project(prune=prune, exclude_paths=exclude_paths)
    except Exception as e:
        print(str(e))
        sys.exit(1)

    print(f"✅ {BCOLORS.OKGREEN}Synced dependencies.{BCOLORS.ENDC}")
    sys.exit(0)


def tach_clean(force: bool = False) -> None:
    logger.info(
        "tach clean called",
        extra={
            "data": LogDataModel(
                function="tach_clean",
                parameters={"force": force},
            ),
        },
    )
    print(
        f"{BCOLORS.WARNING}This will DELETE all existing configuration for {TOOL_NAME}.{BCOLORS.ENDC}"
    )
    root = fs.find_project_config_root(".") or "."
    print(
        f"{BCOLORS.WARNING}Deletion will occur for project with root: '{os.path.abspath(root)}'{BCOLORS.ENDC}"
    )

    if force:
        # No confirmation needed if 'force' passed
        confirmed = True
    else:
        response = input(f"{BCOLORS.OKCYAN}Confirm deletion [y/N]? {BCOLORS.ENDC}: ")
        confirmed = response.lower() in ["y", "yes"]

    if confirmed:
        clean_project(root)
        return
    else:
        print(f"{BCOLORS.OKCYAN}Not deleting configuration.{BCOLORS.ENDC}")


class InstallTarget(Enum):
    PRE_COMMIT = "pre-commit"

    @classmethod
    def choices(cls) -> list[str]:
        return [item.value for item in cls]


def tach_install(path: str, target: InstallTarget, project_root: str = "") -> None:
    logger.info(
        "tach install called",
        extra={
            "data": LogDataModel(
                function="tach_install",
            ),
        },
    )
    try:
        if target == InstallTarget.PRE_COMMIT:
            installed, warning = install_pre_commit(
                path=path, project_root=project_root
            )
        else:
            raise NotImplementedError(f"Target {target} is not supported by 'install'.")
    except Exception as e:
        print(str(e))
        sys.exit(1)

    if installed:
        print(
            f"✅ {BCOLORS.OKGREEN}Pre-commit hook installed to '.git/hooks/pre-commit'.{BCOLORS.ENDC}"
        )
        sys.exit(0)
    else:
        print(
            f"{BCOLORS.WARNING}Pre-commit hook could not be installed: {warning} {BCOLORS.ENDC}"
        )
        sys.exit(1)


def main() -> None:
    args, parser = parse_arguments(sys.argv[1:])
    exclude_paths = args.exclude.split(",") if getattr(args, "exclude", None) else None
    if args.command == "pkg":
        tach_pkg(depth=args.depth, exclude_paths=exclude_paths)
    elif args.command == "sync":
        tach_sync(prune=args.prune, exclude_paths=exclude_paths)
    elif args.command == "check":
        start_spinner("Scanning...")
        tach_check(root=args.root, exact=args.exact, exclude_paths=exclude_paths)
    elif args.command == "clean":
        tach_clean(force=args.force)
    elif args.command == "install":
        try:
            install_target = InstallTarget(args.target)
        except ValueError:
            print(f"{args.target} is not a valid installation target.")
            sys.exit(1)
        tach_install(
            path=args.path, target=install_target, project_root=args.project_root
        )
    else:
        print("Unrecognized command")
        parser.print_help()
        exit(1)


if __name__ == "__main__":
    main()
