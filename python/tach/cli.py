from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path
from typing import IO, TYPE_CHECKING, Any

from tach import __version__, cache
from tach import filesystem as fs
from tach.check_external import check_external
from tach.colors import BCOLORS
from tach.constants import CONFIG_FILE_NAME, TOOL_NAME
from tach.errors import TachError
from tach.extension import (
    ProjectConfig,
    TachCircularDependencyError,
    check,
    check_computation_cache,
    create_computation_cache_key,
    sync_dependency_constraints,
    update_computation_cache,
)
from tach.filesystem import install_pre_commit
from tach.logging import LogDataModel, logger
from tach.parsing import parse_project_config
from tach.report import external_dependency_report, report
from tach.show import generate_module_graph_dot_file, generate_show_url
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
        f"❌ {BCOLORS.FAIL}{error_location}{BCOLORS.ENDC}{BCOLORS.FAIL}: "
        f"{{message}} {BCOLORS.ENDC}"
    )
    warning_template = (
        f"‼️ {BCOLORS.FAIL}{error_location}{BCOLORS.ENDC}{BCOLORS.WARNING}: "
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
    sorted_results = sorted(error_list, key=lambda e: e.file_path)
    for error in sorted_results:
        print(
            build_error_message(error, source_roots=source_roots),
            file=sys.stderr,
        )
    if not all(error.error_info.is_deprecated() for error in sorted_results):
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
        if dependencies:
            print(
                f"❌ {BCOLORS.FAIL}Undeclared dependencies in {BCOLORS.ENDC}{BCOLORS.WARNING}'{file_path}'{BCOLORS.ENDC}:"
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
                f"! {BCOLORS.WARNING}Unused dependencies from project at {BCOLORS.OKCYAN}'{pyproject_path}'{BCOLORS.ENDC}{BCOLORS.ENDC}:"
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
        help="Raise errors if any dependency constraints are unused.",
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
        help="Create a report of dependencies and usages of the given path or directory.",
        description="Create a report of dependencies and usages of the given path or directory.",
    )
    report_parser.add_argument(
        "path", help="The path or directory path used to generate the report."
    )
    report_parser.add_argument(
        "-d",
        "--dependencies",
        required=False,
        type=str,
        metavar="module_path,...",
        help="Comma separated module list of dependencies to include [includes everything by default]",
    )
    report_parser.add_argument(
        "--no-deps",
        action="store_true",
        help="Do not include dependencies in the report.",
    )
    report_parser.add_argument(
        "-u",
        "--usages",
        required=False,
        type=str,
        metavar="module_path,...",
        help="Comma separated module list of usages to include [includes everything by default]",
    )
    report_parser.add_argument(
        "--no-usages", action="store_true", help="Do not include usages in the report."
    )
    add_base_arguments(report_parser)

    ## tach report-external
    report_external_parser = subparsers.add_parser(
        "report-external",
        prog=f"{TOOL_NAME} report-external",
        help="Create a report of third-party dependencies.",
        description="Create a report of third-party dependencies.",
    )
    report_external_parser.add_argument(
        "path", help="The path or directory path used to generate the report."
    )
    report_external_parser.add_argument(
        "--raw",
        action="store_true",
        help="Print machine-readable raw output. Each line will contain a PEP 508 dependency string.",
    )

    ## tach show
    show_parser = subparsers.add_parser(
        "show",
        prog=f"{TOOL_NAME} show",
        help="Visualize the dependency graph of your project.",
        description="Visualize the dependency graph of your project.",
    )
    show_parser.add_argument(
        "--web",
        action="store_true",
        help="Open your dependency graph in a remote web viewer.",
    )
    show_parser.add_argument(
        "-o",
        "--out",
        type=str,
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
                print(output, end="", file=sys.stdout)
            elif fd == 2:
                print(output, end="", file=sys.stderr)


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


class TeeStream:
    def __init__(self, fd: int, source_stream: IO[Any], capture: list[tuple[int, str]]):
        self.fd = fd
        self.source_stream = source_stream
        self.capture = capture

    def write(self, data: Any):
        self.source_stream.write(data)
        self.capture.append((self.fd, data))

    def __getattr__(self, name: str) -> Any:
        # Hack: Proxy attribute access to the source stream
        return getattr(self.source_stream, name)


class Tee:
    def __init__(self):
        # stdout output will be indicated by (1, <data>), stderr output by (2, <data>)
        self.output_capture: list[tuple[int, str]] = []
        self.original_stdout: Any = None
        self.original_stderr: Any = None

    def __enter__(self):
        self.original_stdout = sys.stdout
        self.original_stderr = sys.stderr

        sys.stdout = TeeStream(1, sys.stdout, self.output_capture)
        sys.stderr = TeeStream(2, sys.stderr, self.output_capture)

        return self

    def __exit__(self, exc_type: Any, exc_value: Any, traceback: Any):
        sys.stdout = self.original_stdout
        sys.stderr = self.original_stderr


def extend_and_validate(
    exclude_paths: list[str] | None,
    project_excludes: list[str],
    use_regex_matching: bool,
) -> list[str]:
    if exclude_paths is not None:
        exclude_paths.extend(project_excludes)
    else:
        exclude_paths = project_excludes

    if not use_regex_matching:
        return exclude_paths

    for exclude_path in exclude_paths:
        try:
            re.compile(exclude_path)
        except re.error:
            raise ValueError(
                f"Invalid regex pattern: {exclude_path}. If you meant to use glob matching, set 'use_regex_matching' to false in your .toml file."
            )
    return exclude_paths


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
            print_no_config_found()
            sys.exit(1)

        exact |= project_config.exact

        exclude_paths = extend_and_validate(
            exclude_paths, project_config.exclude, project_config.use_regex_matching
        )

        check_result = check(
            project_root=project_root,
            project_config=project_config,
            exclude_paths=exclude_paths,
        )

        if check_result.warnings:
            print_warnings(check_result.warnings)

        source_roots = [
            project_root / source_root for source_root in project_config.source_roots
        ]

        if check_result.deprecated_warnings:
            print_errors(
                check_result.deprecated_warnings,
                source_roots=source_roots,
            )
        exit_code = 0

        if check_result.errors:
            print_errors(
                check_result.errors,
                source_roots=source_roots,
            )
            exit_code = 1

        # If we're checking in strict mode, we want to verify that pruning constraints has no effect
        if exact:
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
    except Exception as e:
        if isinstance(e, TachCircularDependencyError):
            print_circular_dependency_error(e.dependencies)
        else:
            print(str(e))
        sys.exit(1)

    if exit_code == 0:
        print(f"✅ {BCOLORS.OKGREEN}All module dependencies validated!{BCOLORS.ENDC}")
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

    print(f"✅ {BCOLORS.OKGREEN}All external dependencies validated!{BCOLORS.ENDC}")
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
            f"✅ {BCOLORS.OKGREEN}Set modules! You may want to run '{TOOL_NAME} sync' "
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


def tach_report(
    project_root: Path,
    path: str,
    include_dependency_modules: list[str] | None = None,
    include_usage_modules: list[str] | None = None,
    skip_dependencies: bool = False,
    skip_usages: bool = False,
    exclude_paths: list[str] | None = None,
):
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
        print_no_config_found()
        sys.exit(1)

    exclude_paths = extend_and_validate(
        exclude_paths, project_config.exclude, project_config.use_regex_matching
    )

    report_path = Path(path)
    try:
        print(
            report(
                project_root,
                report_path,
                project_config=project_config,
                include_dependency_modules=include_dependency_modules,
                include_usage_modules=include_usage_modules,
                skip_dependencies=skip_dependencies,
                skip_usages=skip_usages,
                exclude_paths=exclude_paths,
            )
        )
        sys.exit(0)
    except TachError as e:
        print(f"Report failed: {e}")
        sys.exit(1)


def tach_report_external(
    project_root: Path, path: str, raw: bool, exclude_paths: list[str] | None = None
):
    logger.info(
        "tach report-external called",
        extra={
            "data": LogDataModel(
                function="tach_report_external",
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
        print(
            external_dependency_report(
                project_root,
                report_path,
                raw=raw,
                project_config=project_config,
                exclude_paths=exclude_paths,
            )
        )
        sys.exit(0)
    except TachError as e:
        print(f"Report failed: {e}")
        sys.exit(1)


def tach_show(
    project_root: Path, is_web: bool = False, output_filepath: Path | None = None
):
    logger.info(
        "tach show called",
        extra={
            "data": LogDataModel(function="tach_show", parameters={"is_web": is_web}),
        },
    )

    project_config = parse_project_config(root=project_root)
    if project_config is None:
        print_no_config_found()
        sys.exit(1)

    try:
        if is_web:
            result = generate_show_url(project_config)
            if result:
                print("View your dependency graph here:")
                print(result)
                sys.exit(0)
            else:
                sys.exit(1)
        else:
            print_show_web_suggestion()
            output_filepath = output_filepath or Path(f"{TOOL_NAME}_module_graph.dot")
            generate_module_graph_dot_file(project_config, output_filepath)
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

        with Tee() as captured:
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
                value=(captured.output_capture, results.exit_code),
            )
        sys.exit(results.exit_code)
    except TachError as e:
        print(f"{BCOLORS.FAIL}Report failed: {e}{BCOLORS.ENDC}")
        sys.exit(1)


def main() -> None:
    args, parser = parse_arguments(sys.argv[1:])
    project_root = fs.find_project_config_root() or Path.cwd()

    latest_version = cache.get_latest_version()
    if latest_version and latest_version != __version__:
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
            args.dependencies.split(",") if args.dependencies else None
        )
        include_usage_modules = args.usages.split(",") if args.usages else None
        tach_report(
            project_root=project_root,
            path=args.path,
            include_dependency_modules=include_dependency_modules,
            include_usage_modules=include_usage_modules,
            skip_dependencies=args.no_deps,
            skip_usages=args.no_usages,
            exclude_paths=exclude_paths,
        )
    elif args.command == "report-external":
        tach_report_external(
            project_root=project_root,
            path=args.path,
            raw=args.raw,
        )
    elif args.command == "test":
        tach_test(
            project_root=project_root,
            head=args.head,
            base=args.base,
            disable_cache=args.disable_cache,
            pytest_args=args.pytest_args,
        )
    elif args.command == "show":
        tach_show(
            project_root=project_root,
            output_filepath=Path(args.out) if args.out is not None else None,
            is_web=args.web,
        )
    else:
        print("Unrecognized command")
        parser.print_help()
        exit(1)


__all__ = ["main"]
