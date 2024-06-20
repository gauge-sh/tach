from __future__ import annotations

import argparse
import os
import sys
from enum import Enum
from functools import lru_cache
from pathlib import Path
from typing import TYPE_CHECKING

from tach import __version__, cache
from tach import filesystem as fs
from tach.check import BoundaryError, check
from tach.colors import BCOLORS
from tach.constants import CONFIG_FILE_NAME, TOOL_NAME
from tach.core import ProjectConfig
from tach.errors import TachError
from tach.filesystem import install_pre_commit
from tach.logging import LogDataModel, logger
from tach.mod import mod_edit_interactive
from tach.parsing import parse_project_config
from tach.report import report
from tach.sync import prune_dependency_constraints, sync_project

if TYPE_CHECKING:
    from tach.core import UnusedDependencies


class TerminalEnvironment(Enum):
    UNKNOWN = 1
    JETBRAINS = 2
    VSCODE = 3


@lru_cache
def detect_environment() -> TerminalEnvironment:
    if "jetbrains" in os.environ.get("TERMINAL_EMULATOR", "").lower():
        return TerminalEnvironment.JETBRAINS
    elif "vscode" in os.environ.get("TERM_PROGRAM", "").lower():
        return TerminalEnvironment.VSCODE
    return TerminalEnvironment.UNKNOWN


def create_clickable_link(file_path: Path, line: int | None = None) -> str:
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
    elif not error_info.is_dependency_error:
        return error_template.format(message="Unexpected error")

    message = (
        f"Cannot import '{error.import_mod_path}'. "
        f"Tag '{error_info.source_module}' cannot depend on '{error_info.invalid_module}'."
    )

    return error_template.format(message=message)


def print_warnings(warning_list: list[str]) -> None:
    for warning in warning_list:
        print(f"{BCOLORS.WARNING}{warning}{BCOLORS.ENDC}", file=sys.stderr)


def print_errors(error_list: list[BoundaryError]) -> None:
    if not error_list:
        return
    sorted_results = sorted(error_list, key=lambda e: e.file_path)
    for error in sorted_results:
        print(
            build_error_message(error),
            file=sys.stderr,
        )
    print(
        f"{BCOLORS.WARNING}\nIf you intended to add a new dependency, run 'tach sync' to update your module configuration."
        f"\nOtherwise, remove any disallowed imports and consider refactoring.\n{BCOLORS.ENDC}"
    )


def print_unused_dependencies(
    all_unused_dependencies: list[UnusedDependencies],
) -> None:
    constraint_messages = "\n".join(
        f"\t{BCOLORS.WARNING}'{unused_dependencies.path}' does not depend on: {unused_dependencies.dependencies}{BCOLORS.ENDC}"
        for unused_dependencies in all_unused_dependencies
    )
    print(
        f"❌ {BCOLORS.FAIL}Found unused dependencies: {BCOLORS.ENDC}\n"
        + constraint_messages
    )
    print(
        f"{BCOLORS.WARNING}\nRemove the unused dependencies from tach.yml, "
        f"or consider running 'tach sync --prune' to update module configuration and "
        f"eliminate all unused dependencies.\n{BCOLORS.ENDC}"
    )


def print_no_config_yml() -> None:
    print(
        f"{BCOLORS.FAIL} {CONFIG_FILE_NAME}.(yml|yaml) not found{BCOLORS.ENDC}",
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
    parser.add_argument("--version", action="version", version=f"tach {__version__}")

    subparsers = parser.add_subparsers(title="commands", dest="command")
    mod_parser = subparsers.add_parser(
        "mod",
        prog="tach mod",
        help="Configure module boundaries interactively",
        description="Configure module boundaries interactively",
    )
    mod_parser.add_argument(
        "-d",
        "--depth",
        type=int,
        nargs="?",
        default=None,
        help="The number of child directories to expand from the root",
    )
    add_base_arguments(mod_parser)
    check_parser = subparsers.add_parser(
        "check",
        prog="tach check",
        help="Check existing boundaries against your dependencies and module interfaces",
        description="Check existing boundaries against your dependencies and module interfaces",
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
    report_parser = subparsers.add_parser(
        "report",
        prog="tach report",
        help="Create a report of dependencies and usages of the given filepath or directory.",
        description="Create a report of dependencies and usages of the given filepath or directory.",
    )
    report_parser.add_argument(
        "path", help="The filepath or directory path used to generate the report."
    )
    add_base_arguments(report_parser)
    return parser


def parse_arguments(
    args: list[str],
) -> tuple[argparse.Namespace, argparse.ArgumentParser]:
    parser = build_parser()
    parsed_args = parser.parse_args(args)
    return parsed_args, parser


def tach_check(
    project_root: Path,
    exact: bool = False,
    exclude_paths: list[str] | None = None,
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
        project_config = parse_project_config(project_root)
        if project_config is None:
            print_no_config_yml()
            sys.exit(1)

        exact |= project_config.exact

        if exclude_paths is not None and project_config.exclude is not None:
            exclude_paths.extend(project_config.exclude)
        else:
            exclude_paths = project_config.exclude

        check_result = check(
            project_root=project_root,
            project_config=project_config,
            exclude_paths=exclude_paths,
        )
        if check_result.warnings:
            print_warnings(check_result.warnings)

        exit_code = 0

        if check_result.errors:
            print_errors(check_result.errors)
            exit_code = 1

        # If we're checking in strict mode, we want to verify that pruning constraints has no effect
        if exact:
            pruned_config = prune_dependency_constraints(
                project_root=project_root,
                project_config=project_config,
                exclude_paths=exclude_paths,
            )
            unused_dependencies = pruned_config.compare_dependencies(project_config)
            if unused_dependencies:
                print_unused_dependencies(unused_dependencies)
                exit_code = 1
    except Exception as e:
        print(str(e))
        sys.exit(1)

    if exit_code == 0:
        print(f"✅ {BCOLORS.OKGREEN}All module dependencies validated!{BCOLORS.ENDC}")
    sys.exit(exit_code)


def tach_mod(
    project_root: Path, depth: int | None = 1, exclude_paths: list[str] | None = None
):
    logger.info(
        "tach mod called",
        extra={
            "data": LogDataModel(
                function="tach_mod",
                parameters={"depth": depth},
            ),
        },
    )
    try:
        project_config = parse_project_config(root=project_root) or ProjectConfig()
        saved_changes, warnings = mod_edit_interactive(
            project_root=project_root, project_config=project_config, depth=depth
        )
    except Exception as e:
        print(str(e))
        sys.exit(1)

    if warnings:
        print("\n".join(warnings))
    if saved_changes:
        print(
            f"✅ {BCOLORS.OKGREEN}Set modules! You may want to run '{TOOL_NAME} sync' "
            f"to automatically set boundaries.{BCOLORS.ENDC}"
        )
    sys.exit(0)


def tach_sync(
    project_root: Path, prune: bool = False, exclude_paths: list[str] | None = None
):
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
        project_config = parse_project_config(root=project_root)
        if project_config is None:
            print_no_config_yml()
            sys.exit(1)

        if exclude_paths is not None and project_config.exclude is not None:
            exclude_paths.extend(project_config.exclude)
        else:
            exclude_paths = project_config.exclude

        sync_project(
            project_root=project_root,
            project_config=project_config,
            prune=prune,
            exclude_paths=exclude_paths,
        )
    except Exception as e:
        print(str(e))
        sys.exit(1)

    print(f"✅ {BCOLORS.OKGREEN}Synced dependencies.{BCOLORS.ENDC}")
    sys.exit(0)


class InstallTarget(Enum):
    PRE_COMMIT = "pre-commit"

    @classmethod
    def choices(cls) -> list[str]:
        return [item.value for item in cls]


def tach_install(project_root: Path, target: InstallTarget) -> None:
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
            installed, warning = install_pre_commit(project_root=project_root)
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


def tach_report(project_root: Path, path: str, exclude_paths: list[str] | None = None):
    logger.info(
        "tach report called",
        extra={
            "data": LogDataModel(
                function="tach_report",
            ),
        },
    )
    project_config = parse_project_config(root=project_root)
    if project_config is None:
        print_no_config_yml()
        sys.exit(1)

    report_path = Path(path)
    try:
        print(
            report(
                project_root,
                report_path,
                project_config=project_config,
                exclude_paths=exclude_paths,
            )
        )
        sys.exit(0)
    except TachError as e:
        print(f"Report failed: {e}")
        sys.exit(1)


def main() -> None:
    args, parser = parse_arguments(sys.argv[1:])
    project_root = fs.find_project_config_root() or Path.cwd()

    latest_version = cache.get_latest_version()
    if latest_version and latest_version != __version__:
        print(
            f"{BCOLORS.WARNING}WARNING: there is a new tach version available"
            f" ({__version__} -> {latest_version}). Upgrade to remove this warning.{BCOLORS.ENDC}"
        )

    # TODO: rename throughout to 'exclude_patterns' to indicate that these are regex patterns
    exclude_paths = args.exclude.split(",") if getattr(args, "exclude", None) else None

    if args.command == "mod":
        tach_mod(
            project_root=project_root, depth=args.depth, exclude_paths=exclude_paths
        )
    elif args.command == "sync":
        tach_sync(
            project_root=project_root, prune=args.prune, exclude_paths=exclude_paths
        )
    elif args.command == "check":
        tach_check(
            project_root=project_root, exact=args.exact, exclude_paths=exclude_paths
        )
    elif args.command == "install":
        try:
            install_target = InstallTarget(args.target)
        except ValueError:
            print(f"{args.target} is not a valid installation target.")
            sys.exit(1)
        tach_install(project_root=project_root, target=install_target)
    elif args.command == "report":
        tach_report(
            project_root=project_root, path=args.path, exclude_paths=exclude_paths
        )
    else:
        print("Unrecognized command")
        parser.print_help()
        exit(1)


if __name__ == "__main__":
    main()


__all__ = ["main"]
