import argparse
import sys
from enum import Enum
from typing import Optional

from tach.add import add_packages
from tach.check import check, ErrorInfo
from tach import filesystem as fs
from tach.constants import CONFIG_FILE_NAME
from tach.filesystem import install_pre_commit
from tach.init import init_project
from tach.loading import stop_spinner, start_spinner
from tach.parsing import parse_project_config
from tach.colors import BCOLORS


def print_errors(error_list: list[ErrorInfo]) -> None:
    sorted_results = sorted(error_list, key=lambda e: e.location)
    for error in sorted_results:
        print(
            f"❌ {BCOLORS.FAIL}{error.location}{BCOLORS.ENDC}{BCOLORS.WARNING}: {error.message}{BCOLORS.ENDC}",
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
    init_parser = subparsers.add_parser(
        "init",
        prog="tach init",
        help="Initialize boundaries between top-level packages and write dependencies to "
        "`tach.yml`",
        description="Initialize boundaries between top-level packages and write dependencies to "
        "`tach.yml`",
    )
    init_parser.add_argument(
        "-d",
        "--depth",
        type=int,
        nargs="?",
        default=None,
        help="The number of child directories to search for packages to initialize",
    )
    add_base_arguments(init_parser)
    check_parser = subparsers.add_parser(
        "check",
        prog="tach check",
        help="Check existing boundaries against your dependencies and package interfaces",
        description="Check existing boundaries against your dependencies and package interfaces",
    )
    add_base_arguments(check_parser)
    add_parser = subparsers.add_parser(
        "add",
        prog="tach add",
        help="Create a new module boundary around an existing file or folder",
        description="Initialize boundaries between top-level modules and write dependencies to "
        "`tach.yml`",
    )
    add_parser.add_argument(
        "path",
        type=str,
        metavar="file_or_path,...",
        help="The path(s) of the file or directory to create a module boundary around. "
        "Use a comma-separated list for multiple.",
    )
    add_parser.add_argument(
        "-t",
        "--tags",
        required=False,
        type=str,
        metavar="tag,...",
        help="The tag for the module to be initialized with."
        "Use a comma-separated list for multiple.",
    )
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
        help="The path where this installation should occur (default '.')",
    )
    return parser


def parse_arguments(
    args: list[str],
) -> tuple[argparse.Namespace, argparse.ArgumentParser]:
    parser = build_parser()
    parsed_args = parser.parse_args(args)

    if args[0] not in ["init", "add"]:
        fs.validate_project_config_path()

    return parsed_args, parser


def tach_check(
    exclude_paths: Optional[list[str]] = None,
):
    try:
        project_config = parse_project_config()

        if exclude_paths is not None and project_config.exclude is not None:
            exclude_paths.extend(project_config.exclude)
        else:
            exclude_paths = project_config.exclude

        result: list[ErrorInfo] = check(
            ".",
            project_config,
            exclude_paths=exclude_paths,
            exclude_hidden_paths=project_config.exclude_hidden_paths,
        )
    except Exception as e:
        stop_spinner()
        print(str(e))
        sys.exit(1)

    stop_spinner()
    if result:
        print_errors(result)
        sys.exit(1)
    print(f"✅ {BCOLORS.OKGREEN}All package dependencies validated!{BCOLORS.ENDC}")
    sys.exit(0)


def tach_init(depth: Optional[int] = None, exclude_paths: Optional[list[str]] = None):
    try:
        warnings = init_project(root=".", depth=depth, exclude_paths=exclude_paths)
    except Exception as e:
        print(str(e))
        sys.exit(1)

    if warnings:
        print("\n".join(warnings))
    print(f"✅ {BCOLORS.OKGREEN}Initialized '{CONFIG_FILE_NAME}.yml'{BCOLORS.ENDC}")
    sys.exit(0)


def tach_add(paths: set[str], tags: Optional[set[str]] = None) -> None:
    try:
        warnings = add_packages(paths, tags)
    except Exception as e:
        stop_spinner()
        print(str(e))
        sys.exit(1)

    stop_spinner()
    if warnings:
        print("\n".join(warnings))
    if len(paths) > 1:
        print(f"✅ {BCOLORS.OKGREEN}Packages added.{BCOLORS.ENDC}")
    else:
        print(f"✅ {BCOLORS.OKGREEN}Package added.{BCOLORS.ENDC}")
    sys.exit(0)


class InstallTarget(Enum):
    PRE_COMMIT = "pre-commit"

    @classmethod
    def choices(cls) -> list[str]:
        return [item.value for item in cls]


def tach_install(path: str, target: InstallTarget) -> None:
    try:
        if target == InstallTarget.PRE_COMMIT:
            installed, warning = install_pre_commit(path=path)
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
    if args.command == "add":
        paths = set(args.path.split(","))
        tags = set(args.tags.split(",")) if args.tags else None
        tach_add(paths=paths, tags=tags)
        return
    exclude_paths = args.exclude.split(",") if getattr(args, "exclude", None) else None
    if args.command == "init":
        tach_init(depth=args.depth, exclude_paths=exclude_paths)
    elif args.command == "check":
        start_spinner("Scanning...")
        tach_check(exclude_paths=exclude_paths)
    elif args.command == "install":
        try:
            install_target = InstallTarget(args.target)
        except ValueError:
            print(f"{args.target} is not a valid installation target.")
            sys.exit(1)
        tach_install(path=args.path, target=install_target)
    else:
        print("Unrecognized command")
        parser.print_help()
        exit(1)


if __name__ == "__main__":
    main()
