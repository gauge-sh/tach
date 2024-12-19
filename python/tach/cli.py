from __future__ import annotations

import argparse
import sys
from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path
from typing import TYPE_CHECKING, Any

from tach import __version__, cache, icons
from tach import filesystem as fs
from tach.check_external import check_external
from tach.colors import BCOLORS
from tach.constants import CONFIG_FILE_NAME, TOOL_NAME
from tach.errors import (
    TachCircularDependencyError,
    TachClosedBetaError,
    TachError,
    TachSetupError,
    TachVisibilityError,
)
from tach.extension import (
    ProjectConfig,
    check,
    check_computation_cache,
    create_computation_cache_key,
    run_server,
    sync_dependency_constraints,
    update_computation_cache,
)
from tach.filesystem import install_pre_commit
from tach.logging import LogDataModel, logger
from tach.modularity import export_report, upload_report_to_gauge
from tach.parsing import extend_and_validate, parse_project_config
from tach.report import external_dependency_report, report
from tach.show import (
    generate_module_graph_dot_file,
    generate_module_graph_mermaid,
    generate_show_url,
)
from tach.sync import sync_project
from tach.test import run_affected_tests
from tach.utils.display import create_clickable_link

if TYPE_CHECKING:
    from tach.extension import BoundaryError, UnusedDependencies


def build_error_message(error: BoundaryError, source_roots: list[Path]) -> str:
    absolute_error_path = next(
        (
            source_root / error.file_path
            for source_root in source_roots
            if (source_root / error.file_path).exists()
        ),
        None,
    )

    if absolute_error_path is None:
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
        f"{icons.FAIL}  {BCOLORS.FAIL}{error_location}{BCOLORS.ENDC}{BCOLORS.WARNING}: "
        f"{{message}} {BCOLORS.ENDC}"
    )
    warning_template = (
        f"{icons.WARNING}  {BCOLORS.FAIL}{error_location}{BCOLORS.ENDC}{BCOLORS.WARNING}: "
        f"{{message}} {BCOLORS.ENDC}"
    )
    error_info = error.error_info
    if error_info.is_deprecated():
        return warning_template.format(message=error_info.to_pystring())
    return error_template.format(message=error_info.to_pystring())


def print_warnings(warning_list: list[str]) -> None:
    for warning in warning_list:
        print(f"{BCOLORS.WARNING}{warning}{BCOLORS.ENDC}", file=sys.stderr)


def print_errors(error_list: list[BoundaryError], source_roots: list[Path]) -> None:
    if not error_list:
        return

    interface_errors: list[BoundaryError] = []
    dependency_errors: list[BoundaryError] = []
    for error in sorted(error_list, key=lambda e: e.file_path):
        if error.error_info.is_interface_error():
            interface_errors.append(error)
        else:
            dependency_errors.append(error)

    if interface_errors:
        print(f"{BCOLORS.FAIL}Interface Errors:{BCOLORS.ENDC}", file=sys.stderr)
        for error in interface_errors:
            print(
                build_error_message(error, source_roots=source_roots),
                file=sys.stderr,
            )
        print(
            f"{BCOLORS.WARNING}\nIf you intended to change an interface, edit the '[[interfaces]]' section of {CONFIG_FILE_NAME}.toml."
            f"\nOtherwise, remove any disallowed imports and consider refactoring.\n{BCOLORS.ENDC}",
            file=sys.stderr,
        )

    if dependency_errors:
        print(f"{BCOLORS.FAIL}Dependency Errors:{BCOLORS.ENDC}", file=sys.stderr)
        has_real_errors = False
        for error in dependency_errors:
            if not error.error_info.is_deprecated():
                has_real_errors = True
            print(
                build_error_message(error, source_roots=source_roots),
                file=sys.stderr,
            )
        print(file=sys.stderr)
        if has_real_errors:
            print(
                f"{BCOLORS.WARNING}If you intended to add a new dependency, run 'tach sync' to update your module configuration."
                f"\nOtherwise, remove any disallowed imports and consider refactoring.\n{BCOLORS.ENDC}",
                file=sys.stderr,
            )


def print_unused_dependencies(
    all_unused_dependencies: list[UnusedDependencies],
) -> None:
    constraint_messages = "\n".join(
        f"\t{BCOLORS.WARNING}'{unused_dependencies.path}' does not depend on: {[dependency.path for dependency in unused_dependencies.dependencies]}{BCOLORS.ENDC}"
        for unused_dependencies in all_unused_dependencies
    )
    print(
        f"{icons.FAIL}: {BCOLORS.FAIL}Found unused dependencies: {BCOLORS.ENDC}\n"
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


def print_show_web_suggestion(is_mermaid: bool = False) -> None:
    if is_mermaid:
        print(
            f"{BCOLORS.OKCYAN}NOTE: You are generating a Mermaid graph locally representing your module graph. For a remotely hosted visualization, use the '--web' argument.\nTo visualize your graph, you will need to use Mermaid.js: https://mermaid.js.org/config/usage.html\n{BCOLORS.ENDC}"
        )
    else:
        print(
            f"{BCOLORS.OKCYAN}NOTE: You are generating a DOT file locally representing your module graph. For a remotely hosted visualization, use the '--web' argument.\nTo visualize your graph, you will need a program like GraphViz: https://www.graphviz.org/download/\n{BCOLORS.ENDC}"
        )


def print_generated_module_graph_file(
    output_filepath: Path, is_mermaid: bool = False
) -> None:
    if is_mermaid:
        print(
            f"{BCOLORS.OKGREEN}Generated a Mermaid file containing your module graph at '{output_filepath}'{BCOLORS.ENDC}"
        )
    else:
        print(
            f"{BCOLORS.OKGREEN}Generated a DOT file containing your module graph at '{output_filepath}'{BCOLORS.ENDC}"
        )


def print_circular_dependency_error(module_paths: list[str]) -> None:
    print(
        "\n".join(
            [
                f"{icons.FAIL} {BCOLORS.FAIL}Circular dependency detected for module {BCOLORS.ENDC}'{module_path}'"
                for module_path in module_paths
            ]
        )
        + f"\n\n{BCOLORS.WARNING}Resolve circular dependencies.\n"
        f"Remove or unset 'forbid_circular_dependencies' from "
        f"'{CONFIG_FILE_NAME}.toml' to allow circular dependencies.{BCOLORS.ENDC}"
    )


def print_visibility_errors(
    visibility_errors: list[tuple[str, str, list[str]]],
) -> None:
    for dependent_module, dependency_module, visibility in visibility_errors:
        print(
            f"{icons.FAIL} {BCOLORS.FAIL}Module configuration error:{BCOLORS.ENDC} {BCOLORS.WARNING}'{dependent_module}' cannot depend on '{dependency_module}' because '{dependent_module}' does not match its visibility: {visibility}.{BCOLORS.ENDC}"
            "\n"
            f"{BCOLORS.WARNING}Adjust 'visibility' for '{dependency_module}' to include '{dependent_module}', or remove the dependency.{BCOLORS.ENDC}"
            "\n"
        )


def print_undeclared_dependencies(
    undeclared_dependencies: dict[str, list[str]],
) -> None:
    for file_path, dependencies in undeclared_dependencies.items():
        if dependencies:
            print(
                f"{icons.FAIL}: {BCOLORS.FAIL}Undeclared dependencies in {BCOLORS.ENDC}{BCOLORS.WARNING}'{file_path}'{BCOLORS.ENDC}:"
            )
            for dependency in dependencies:
                print(f"\t{BCOLORS.FAIL}{dependency}{BCOLORS.ENDC}")
    print(
        f"{BCOLORS.WARNING}\nAdd the undeclared dependencies to the corresponding pyproject.toml file, "
        f"or consider ignoring the dependencies by adding them to the 'external.exclude' list in {CONFIG_FILE_NAME}.toml.\n{BCOLORS.ENDC}"
    )


def print_unused_external_dependencies(
    unused_dependencies: dict[str, list[str]],
) -> None:
    for pyproject_path, dependencies in unused_dependencies.items():
        if dependencies:
            print(
                f"{icons.WARNING}  {BCOLORS.WARNING}Unused dependencies from project at {BCOLORS.OKCYAN}'{pyproject_path}'{BCOLORS.ENDC}{BCOLORS.ENDC}:"
            )
            for dependency in dependencies:
                print(f"\t{BCOLORS.WARNING}{dependency}{BCOLORS.ENDC}")
    print(
        f"{BCOLORS.OKCYAN}\nRemove the unused dependencies from the corresponding pyproject.toml file, "
        f"or consider ignoring the dependencies by adding them to the 'external.exclude' list in {CONFIG_FILE_NAME}.toml.\n{BCOLORS.ENDC}"
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
        prog=TOOL_NAME,
        add_help=True,
    )
    parser.add_argument(
        "--version", action="version", version=f"{TOOL_NAME} {__version__}"
    )

    subparsers = parser.add_subparsers(title="commands", dest="command")

    ## tach mod
    mod_parser = subparsers.add_parser(
        "mod",
        prog=f"{TOOL_NAME} mod",
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

    ## tach check
    check_parser = subparsers.add_parser(
        "check",
        prog=f"{TOOL_NAME} check",
        help="Check existing boundaries against your dependencies and module interfaces",
        description="Check existing boundaries against your dependencies and module interfaces",
    )
    check_parser.add_argument(
        "--exact",
        action="store_true",
        help="When checking dependencies, raise errors if any dependencies are unused.",
    )
    check_parser.add_argument(
        "--dependencies",
        action="store_true",
        help="Check dependency constraints between modules. When present, all checks must be explicitly enabled.",
    )
    check_parser.add_argument(
        "--interfaces",
        action="store_true",
        help="Check interface implementations. When present, all checks must be explicitly enabled.",
    )
    add_base_arguments(check_parser)

    ## tach check-external
    check_parser_external = subparsers.add_parser(
        "check-external",
        prog=f"{TOOL_NAME} check-external",
        help="Perform checks related to third-party dependencies",
        description="Perform checks related to third-party dependencies",
    )
    add_base_arguments(check_parser_external)

    ## tach sync
    sync_parser = subparsers.add_parser(
        "sync",
        prog=f"{TOOL_NAME} sync",
        help="Sync constraints with actual dependencies in your project.",
        description="Sync constraints with actual dependencies in your project.",
    )
    sync_parser.add_argument(
        "--add",
        action="store_true",
        help="Add any missing dependencies, but do not remove unused dependencies.",
    )
    add_base_arguments(sync_parser)

    ## tach report
    report_parser = subparsers.add_parser(
        "report",
        prog=f"{TOOL_NAME} report",
        help="Create a report of dependencies and usages.",
        description="Create a report of dependencies and usages.",
    )
    report_parser.add_argument(
        "path", help="The path or directory path used to generate the report."
    )
    # Report type flags
    report_parser.add_argument(
        "--dependencies",
        action="store_true",
        help="Generate dependency report. When present, all reports must be explicitly enabled.",
    )
    report_parser.add_argument(
        "--usages",
        action="store_true",
        help="Generate usage report. When present, all reports must be explicitly enabled.",
    )
    report_parser.add_argument(
        "--external",
        action="store_true",
        help="Generate external dependency report. When present, all reports must be explicitly enabled.",
    )
    # Report options
    report_parser.add_argument(
        "-d",
        "--dependency-modules",
        required=False,
        type=str,
        metavar="module_path,...",
        help="Comma separated module list of dependencies to include [includes everything by default]",
    )
    report_parser.add_argument(
        "-u",
        "--usage-modules",
        required=False,
        type=str,
        metavar="module_path,...",
        help="Comma separated module list of usages to include [includes everything by default]",
    )
    report_parser.add_argument(
        "--raw",
        action="store_true",
        help="Group lines by module and print each without any formatting.",
    )
    add_base_arguments(report_parser)

    ## tach show
    show_parser = subparsers.add_parser(
        "show",
        prog=f"{TOOL_NAME} show",
        help="Visualize the dependency graph of your project.",
        description="Visualize the dependency graph of your project.",
    )
    show_parser.add_argument(
        "included_paths",
        type=Path,
        nargs="*",
        help="Paths to include in the module graph. If not provided, the entire project is included.",
    )
    show_parser.add_argument(
        "--web",
        action="store_true",
        help="Open your dependency graph in a remote web viewer.",
    )
    show_parser.add_argument(
        "--mermaid",
        action="store_true",
        help="Generate a mermaid.js graph instead of a DOT file.",
    )
    show_parser.add_argument(
        "-o",
        "--out",
        type=Path,
        nargs="?",
        default=None,
        help="Specify an output path for a locally generated module graph file.",
    )

    ## tach install
    install_parser = subparsers.add_parser(
        "install",
        prog=f"{TOOL_NAME} install",
        help=f"Install {TOOL_NAME} into your workflow (e.g. as a pre-commit hook)",
        description=f"Install {TOOL_NAME} into your workflow (e.g. as a pre-commit hook)",
    )
    install_parser.add_argument(
        "target",
        choices=InstallTarget.choices(),
        help="What kind of installation to perform (e.g. pre-commit)",
    )

    ## tach test
    test_parser = subparsers.add_parser(
        "test",
        prog=f"{TOOL_NAME} test",
        help="Run tests on modules impacted by the current changes.",
        description="Run tests on modules impacted by the current changes.",
    )
    test_parser.add_argument(
        "--base",
        type=str,
        nargs="?",
        default="main",
        help="The base commit to use when determining which modules are impacted by changes. [default: 'main']",
    )
    test_parser.add_argument(
        "--head",
        type=str,
        nargs="?",
        default="",
        help="The head commit to use when determining which modules are impacted by changes. [default: current filesystem]",
    )
    test_parser.add_argument(
        "--disable-cache",
        action="store_true",
        help="Do not check cache for results, and do not push results to cache.",
    )
    test_parser.add_argument(
        "pytest_args",
        nargs=argparse.REMAINDER,
        help=f"Arguments forwarded to pytest. Use '--' to separate these arguments. Ex: '{TOOL_NAME} test -- -v'",
    )

    ## tach upload
    upload_parser = subparsers.add_parser(
        "upload",
        prog=f"{TOOL_NAME} upload",
        help="[CLOSED BETA] Upload a modularity report to Gauge",
        description="[CLOSED BETA] Upload a modularity report to Gauge",
    )
    upload_parser.add_argument(
        "-f",
        "--force",
        action="store_true",
        help="Ignore warnings and force the report to be generated.",
    )

    ## tach export
    export_parser = subparsers.add_parser(
        "export",
        prog=f"{TOOL_NAME} export",
        help="Export a modularity report to a local file",
        description="Export a modularity report to a local file",
    )
    export_parser.add_argument(
        "-o",
        "--output",
        type=Path,
        nargs="?",
        default=None,
        help="Specify an output path for the modularity report [DEFAULT: 'modularity_report.json']",
    )
    export_parser.add_argument(
        "-f",
        "--force",
        action="store_true",
        help="Ignore warnings and force the report to be generated.",
    )

    ## tach server
    subparsers.add_parser(
        "server",
        prog=f"{TOOL_NAME} server",
        help="Start the Language Server Protocol (LSP) server",
        description="Start the Language Server Protocol (LSP) server",
    )

    return parser


def parse_arguments(
    args: list[str],
) -> tuple[argparse.Namespace, argparse.ArgumentParser]:
    parser = build_parser()
    parsed_args = parser.parse_args(args)
    return parsed_args, parser


@dataclass
class CachedOutput:
    key: str
    output: list[tuple[int, str]] = field(default_factory=list)
    exit_code: int | None = None

    @property
    def exists(self) -> bool:
        return self.exit_code is not None

    def replay(self):
        for fd, output in self.output:
            if fd == 1:
                print(output, file=sys.stdout)
            elif fd == 2:
                print(output, file=sys.stderr)


def check_cache_for_action(
    project_root: Path, project_config: ProjectConfig, action: str
) -> CachedOutput:
    cache_key = create_computation_cache_key(
        project_root=str(project_root),
        source_roots=[
            str(project_root / source_root)
            for source_root in project_config.source_roots
        ],
        action=action,
        py_interpreter_version=f"{sys.version_info.major}.{sys.version_info.minor}.{sys.version_info.micro}",
        file_dependencies=project_config.cache.file_dependencies,
        env_dependencies=project_config.cache.env_dependencies,
        backend=project_config.cache.backend,
    )
    cache_result = check_computation_cache(
        project_root=str(project_root), cache_key=cache_key
    )
    if cache_result:
        return CachedOutput(
            key=cache_key,
            output=cache_result[0],
            exit_code=cache_result[1],
        )
    return CachedOutput(key=cache_key)


def tach_check(
    project_root: Path,
    exact: bool = False,
    dependencies: bool = True,
    interfaces: bool = True,
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
            print_no_config_found()
            sys.exit(1)

        exact |= project_config.exact

        exclude_paths = extend_and_validate(
            exclude_paths, project_config.exclude, project_config.use_regex_matching
        )

        check_result = check(
            project_root=project_root,
            project_config=project_config,
            dependencies=dependencies,
            interfaces=interfaces,
            exclude_paths=exclude_paths,
        )

        if check_result.warnings:
            print_warnings(check_result.warnings)

        source_roots = [
            project_root / source_root for source_root in project_config.source_roots
        ]

        print_errors(
            check_result.errors + check_result.deprecated_warnings,
            source_roots=source_roots,
        )
        exit_code = 1 if len(check_result.errors) > 0 else 0

        # If we're checking in exact mode, we want to verify that pruning constraints has no effect
        if dependencies and exact:
            pruned_config = sync_dependency_constraints(
                project_root=project_root,
                project_config=project_config,
                exclude_paths=exclude_paths,
                prune=True,
            )
            unused_dependencies = pruned_config.compare_dependencies(project_config)
            if unused_dependencies:
                print_unused_dependencies(unused_dependencies)
                exit_code = 1
    except TachCircularDependencyError as e:
        print_circular_dependency_error(e.dependencies)
        sys.exit(1)
    except TachVisibilityError as e:
        print_visibility_errors(e.visibility_errors)
        sys.exit(1)
    except Exception as e:
        print(str(e))
        sys.exit(1)

    if exit_code == 0:
        print(f"{icons.SUCCESS} {BCOLORS.OKGREEN}All modules validated!{BCOLORS.ENDC}")
    sys.exit(exit_code)


def tach_check_external(project_root: Path, exclude_paths: list[str] | None = None):
    logger.info(
        "tach check-external called",
        extra={
            "data": LogDataModel(
                function="tach_check_external",
            ),
        },
    )
    try:
        project_config = parse_project_config(project_root)
        if project_config is None:
            print_no_config_found()
            sys.exit(1)

        exclude_paths = extend_and_validate(
            exclude_paths, project_config.exclude, project_config.use_regex_matching
        )

        result = check_external(
            project_root=project_root,
            project_config=project_config,
            exclude_paths=exclude_paths,
        )

        if result.unused_dependencies:
            print_unused_external_dependencies(result.unused_dependencies)

        if result.undeclared_dependencies:
            print_undeclared_dependencies(result.undeclared_dependencies)
            sys.exit(1)

    except Exception as e:
        print(str(e))
        sys.exit(1)

    print(
        f"{icons.SUCCESS} {BCOLORS.OKGREEN}All external dependencies validated!{BCOLORS.ENDC}"
    )
    sys.exit(0)


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
    # Local import because prompt_toolkit takes about ~80ms to load
    from tach.mod import mod_edit_interactive

    try:
        project_config = parse_project_config(root=project_root) or ProjectConfig()
        exclude_paths = extend_and_validate(
            exclude_paths, project_config.exclude, project_config.use_regex_matching
        )
        saved_changes, warnings = mod_edit_interactive(
            project_root=project_root,
            project_config=project_config,
            exclude_paths=exclude_paths,
            depth=depth,
        )
    except Exception as e:
        print(str(e))
        sys.exit(1)

    if warnings:
        print("\n".join(warnings))
    if saved_changes:
        print(
            f"{icons.SUCCESS} {BCOLORS.OKGREEN}Set modules! You may want to run '{TOOL_NAME} sync' "
            f"to automatically set boundaries.{BCOLORS.ENDC}"
        )
    sys.exit(0)


def tach_sync(
    project_root: Path, add: bool = False, exclude_paths: list[str] | None = None
):
    logger.info(
        "tach sync called",
        extra={
            "data": LogDataModel(
                function="tach_sync",
                parameters={"add": add},
            ),
        },
    )
    try:
        project_config = parse_project_config(root=project_root)
        if project_config is None:
            print_no_config_found()
            sys.exit(1)

        exclude_paths = extend_and_validate(
            exclude_paths, project_config.exclude, project_config.use_regex_matching
        )

        sync_project(
            project_root=project_root,
            project_config=project_config,
            exclude_paths=exclude_paths,
            add=add,
        )
    except Exception as e:
        print(str(e))
        sys.exit(1)

    print(f"{icons.SUCCESS} {BCOLORS.OKGREEN}Synced dependencies.{BCOLORS.ENDC}")
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
            f"{icons.SUCCESS} {BCOLORS.OKGREEN}Pre-commit hook installed to '.git/hooks/pre-commit'.{BCOLORS.ENDC}"
        )
        sys.exit(0)
    else:
        print(
            f"{BCOLORS.WARNING}Pre-commit hook could not be installed: {warning} {BCOLORS.ENDC}"
        )
        sys.exit(1)


def tach_report(
    project_root: Path,
    path: str,
    include_dependency_modules: list[str] | None = None,
    include_usage_modules: list[str] | None = None,
    dependencies: bool = False,
    usages: bool = False,
    external: bool = False,
    raw: bool = False,
    exclude_paths: list[str] | None = None,
):
    logger.info(
        "tach report called",
        extra={
            "data": LogDataModel(
                function="tach_report",
                parameters={
                    "dependencies": dependencies,
                    "usages": usages,
                    "external": external,
                },
            ),
        },
    )
    project_config = parse_project_config(root=project_root)
    if project_config is None:
        print_no_config_found()
        sys.exit(1)

    exclude_paths = extend_and_validate(
        exclude_paths, project_config.exclude, project_config.use_regex_matching
    )

    report_path = Path(path)
    try:
        # Generate reports based on flags
        generate_all = not (dependencies or usages or external)
        generate_dependencies = generate_all or dependencies
        generate_usages = generate_all or usages
        generate_external = generate_all or external

        reports: list[str] = []
        if generate_dependencies or generate_usages:
            reports.append(
                report(
                    project_root,
                    report_path,
                    project_config=project_config,
                    include_dependency_modules=include_dependency_modules,
                    include_usage_modules=include_usage_modules,
                    skip_dependencies=not generate_dependencies,
                    skip_usages=not generate_usages,
                    raw=raw,
                    exclude_paths=exclude_paths,
                )
            )

        if generate_external:
            reports.append(
                external_dependency_report(
                    project_root,
                    report_path,
                    raw=raw,
                    project_config=project_config,
                    exclude_paths=exclude_paths,
                )
            )

        print("\n".join(reports))
        sys.exit(0)
    except TachError as e:
        print(f"Report failed: {e}")
        sys.exit(1)


def tach_show(
    project_root: Path,
    included_paths: list[Path] | None = None,
    is_web: bool = False,
    is_mermaid: bool = False,
    output_filepath: Path | None = None,
):
    logger.info(
        "tach show called",
        extra={
            "data": LogDataModel(
                function="tach_show",
                parameters={"is_web": is_web, "is_mermaid": is_mermaid},
            ),
        },
    )

    if is_web and is_mermaid:
        print(
            f"{BCOLORS.WARNING}Passing --web always generates a Mermaid graph remotely; ignoring '--mermaid' flag.{BCOLORS.ENDC}"
        )

    project_config = parse_project_config(root=project_root)
    if project_config is None:
        print_no_config_found()
        sys.exit(1)

    try:
        if is_web:
            result = generate_show_url(
                project_root, project_config, included_paths=included_paths
            )
            if result:
                print("View your dependency graph here:")
                print(result)
                sys.exit(0)
            else:
                sys.exit(1)
        else:
            print_show_web_suggestion(is_mermaid=is_mermaid)
            if is_mermaid:
                output_filepath = output_filepath or Path(
                    f"{TOOL_NAME}_module_graph.mmd"
                )
                generate_module_graph_mermaid(
                    project_root,
                    project_config,
                    included_paths=included_paths,
                    output_filepath=output_filepath,
                )
                print_generated_module_graph_file(output_filepath, is_mermaid=True)
                sys.exit(0)
            else:
                output_filepath = output_filepath or Path(
                    f"{TOOL_NAME}_module_graph.dot"
                )
                generate_module_graph_dot_file(
                    project_root,
                    project_config,
                    included_paths=included_paths,
                    output_filepath=output_filepath,
                )
                print_generated_module_graph_file(output_filepath)
                sys.exit(0)
    except TachError as e:
        print(f"Failed to show module graph: {e}")
        sys.exit(1)


def tach_test(
    project_root: Path,
    head: str,
    base: str,
    disable_cache: bool,
    pytest_args: list[Any],
):
    logger.info(
        "tach test called",
        extra={
            "data": LogDataModel(
                function="tach_test",
                parameters={
                    "disable_cache": disable_cache,
                    "pytest_args": pytest_args,
                },
            ),
        },
    )
    project_config = parse_project_config(root=project_root)
    if project_config is None:
        print_no_config_found()
        sys.exit(1)

    if pytest_args and pytest_args[0] != "--":
        print(
            f"{BCOLORS.FAIL}Unknown arguments received. Use '--' to separate arguments for pytest. Ex: '{TOOL_NAME} test -- -v'{BCOLORS.ENDC}"
        )
        sys.exit(1)

    try:
        if disable_cache:
            # If cache disabled, just run affected tests and exit
            results = run_affected_tests(
                project_root=project_root,
                project_config=project_config,
                head=head,
                base=base,
                pytest_args=pytest_args[1:],  # Remove '--' pseudo-argument
            )
            sys.exit(results.exit_code)

        # Below this line caching is enabled
        cached_output = check_cache_for_action(
            project_root, project_config, f"tach-test,{head},{base},{pytest_args}"
        )
        if cached_output.exists:
            # Early exit, cached terminal output was found
            print(
                f"{BCOLORS.OKGREEN}============ Cached results found!  ============{BCOLORS.ENDC}"
            )
            cached_output.replay()
            print(
                f"{BCOLORS.OKGREEN}============ END Cached results  ============{BCOLORS.ENDC}"
            )
            sys.exit(cached_output.exit_code)

        # Cache missed, capture terminal output while tests run so we can update the cache

        results = run_affected_tests(
            project_root=project_root,
            project_config=project_config,
            head=head,
            base=base,
            pytest_args=pytest_args[1:],  # Remove '--' pseudo-argument
        )

        if results.tests_ran_to_completion:
            update_computation_cache(
                str(project_root),
                cache_key=cached_output.key,
                value=(
                    [
                        *(
                            (1, stdout_line)
                            for stdout_line in results.stdout.split("\n")
                        ),
                        *(
                            (2, stderr_line)
                            for stderr_line in results.stderr.split("\n")
                        ),
                    ],
                    results.exit_code,
                ),
            )
        sys.exit(results.exit_code)
    except TachError as e:
        print(f"{BCOLORS.FAIL}Report failed: {e}{BCOLORS.ENDC}")
        sys.exit(1)


def tach_export(
    project_root: Path,
    output_path: Path | None = None,
    force: bool = False,
):
    logger.info(
        "tach export called",
        extra={
            "data": LogDataModel(
                function="tach_export",
                parameters={"force": force},
            ),
        },
    )

    project_config = parse_project_config(root=project_root)
    if project_config is None:
        print_no_config_found()
        sys.exit(1)

    try:
        export_report(
            project_root=project_root,
            project_config=project_config,
            output_path=output_path,
            force=force,
        )
    except TachError as e:
        print(f"Failed to export modularity report: {e}")
        sys.exit(1)


def tach_upload(
    project_root: Path,
    force: bool = False,
):
    logger.info(
        "tach upload called",
        extra={
            "data": LogDataModel(
                function="tach_upload",
                parameters={"force": force},
            ),
        },
    )

    project_config = parse_project_config(root=project_root)
    if project_config is None:
        print_no_config_found()
        sys.exit(1)

    try:
        upload_report_to_gauge(
            project_root=project_root,
            project_config=project_config,
            force=force,
        )
    except TachClosedBetaError as e:
        print(e)
        sys.exit(1)
    except TachError as e:
        print(f"Failed to upload modularity report: {e}")
        sys.exit(1)


def tach_server(project_root: Path):
    logger.info(
        "tach server called",
        extra={
            "data": LogDataModel(function="tach_server"),
        },
    )
    project_config = parse_project_config(root=project_root)
    if project_config is None:
        print_no_config_found()
        sys.exit(1)

    try:
        run_server(project_root, project_config)
    except TachSetupError as e:
        print(f"Failed to setup LSP server: {e}")
        sys.exit(1)


def current_version_is_behind(latest_version: str) -> bool:
    try:
        current_version_parts = list(map(int, __version__.split(".")[:3]))
        latest_version_parts = list(map(int, latest_version.split(".")[:3]))
        return current_version_parts < latest_version_parts
    except Exception:
        return False


def main() -> None:
    args, parser = parse_arguments(sys.argv[1:])
    project_root = fs.find_project_config_root() or Path.cwd()

    latest_version = cache.get_latest_version()
    if latest_version and current_version_is_behind(latest_version):
        print(
            f"{BCOLORS.WARNING}WARNING: there is a new {TOOL_NAME} version available"
            f" ({__version__} -> {latest_version}). Upgrade to remove this warning.{BCOLORS.ENDC}"
        )

    exclude_paths = args.exclude.split(",") if getattr(args, "exclude", None) else None

    if args.command == "mod":
        tach_mod(
            project_root=project_root, depth=args.depth, exclude_paths=exclude_paths
        )
    elif args.command == "sync":
        tach_sync(project_root=project_root, add=args.add, exclude_paths=exclude_paths)
    elif args.command == "check":
        if args.dependencies or args.interfaces:
            tach_check(
                project_root=project_root,
                dependencies=args.dependencies,
                interfaces=args.interfaces,
                exact=args.exact,
                exclude_paths=exclude_paths,
            )
        else:
            tach_check(
                project_root=project_root, exact=args.exact, exclude_paths=exclude_paths
            )
    elif args.command == "check-external":
        tach_check_external(project_root=project_root, exclude_paths=exclude_paths)
    elif args.command == "install":
        try:
            install_target = InstallTarget(args.target)
        except ValueError:
            print(f"{args.target} is not a valid installation target.")
            sys.exit(1)
        tach_install(project_root=project_root, target=install_target)
    elif args.command == "report":
        include_dependency_modules = (
            args.dependency_modules.split(",") if args.dependency_modules else None
        )
        include_usage_modules = (
            args.usage_modules.split(",") if args.usage_modules else None
        )
        tach_report(
            project_root=project_root,
            path=args.path,
            include_dependency_modules=include_dependency_modules,
            include_usage_modules=include_usage_modules,
            dependencies=args.dependencies,
            usages=args.usages,
            external=args.external,
            raw=args.raw,
            exclude_paths=exclude_paths,
        )
    elif args.command == "show":
        tach_show(
            project_root=project_root,
            included_paths=args.included_paths,
            output_filepath=args.out,
            is_web=args.web,
            is_mermaid=args.mermaid,
        )
    elif args.command == "test":
        tach_test(
            project_root=project_root,
            head=args.head,
            base=args.base,
            disable_cache=args.disable_cache,
            pytest_args=args.pytest_args,
        )
    elif args.command == "export":
        tach_export(
            project_root=project_root,
            output_path=args.output,
            force=args.force,
        )
    elif args.command == "upload":
        tach_upload(
            project_root=project_root,
            force=args.force,
        )
    elif args.command == "server":
        tach_server(project_root=project_root)
    else:
        print("Unrecognized command")
        parser.print_help()
        exit(1)


__all__ = ["main"]
