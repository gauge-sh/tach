import argparse
import sys
from typing import Optional

from modguard.add import add_packages
from modguard.check import check, ErrorInfo
from modguard import filesystem as fs
from modguard.init import init_project
from modguard.loading import stop_spinner, start_spinner
from modguard.parsing import parse_project_config
from modguard.colors import BCOLORS


def print_errors(error_list: list[ErrorInfo]) -> None:
    sorted_results = sorted(error_list, key=lambda e: e.location)
    for error in sorted_results:
        print(
            f"❌ {BCOLORS.FAIL}{error.location}{BCOLORS.WARNING}: {error.message}",
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
        prog="modguard",
        add_help=True,
        epilog="Make sure modguard is run from the root of your Python project,"
        " and `modguard.yml` is present",
    )
    subparsers = parser.add_subparsers(title="commands", dest="command")
    init_parser = subparsers.add_parser(
        "init",
        prog="modguard init",
        help="Initialize boundaries between top-level packages and write dependencies to "
        "`modguard.yml`",
        description="Initialize boundaries between top-level packages and write dependencies to "
        "`modguard.yml`",
    )
    add_base_arguments(init_parser)
    check_parser = subparsers.add_parser(
        "check",
        prog="modguard check",
        help="Check existing boundaries against your dependencies and package interfaces",
        description="Check existing boundaries against your dependencies and package interfaces",
    )
    add_base_arguments(check_parser)
    add_parser = subparsers.add_parser(
        "add",
        prog="modguard add",
        help="Create a new module boundary around an existing file or folder",
        description="Initialize boundaries between top-level modules and write dependencies to "
        "`modguard.yml`",
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
    return parser


def parse_arguments(
    args: list[str],
) -> tuple[argparse.Namespace, argparse.ArgumentParser]:
    parser = build_parser()
    parsed_args = parser.parse_args(args)

    if args[0] not in ["init", "add"]:
        fs.validate_project_config_path()

    return parsed_args, parser


def modguard_check(
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
    print(f"✅ {BCOLORS.OKGREEN}All packages safely guarded!")
    sys.exit(0)


def modguard_init(exclude_paths: Optional[list[str]] = None):
    try:
        warnings = init_project(root=".", exclude_paths=exclude_paths)
    except Exception as e:
        stop_spinner()
        print(str(e))
        sys.exit(1)

    stop_spinner()
    if warnings:
        print("\n".join(warnings))
    print(f"✅ {BCOLORS.OKGREEN}Modguard initialized.")
    sys.exit(0)


def modguard_add(paths: set[str], tags: Optional[set[str]] = None) -> None:
    try:
        warnings = add_packages(paths, tags)
    except Exception as e:
        stop_spinner()
        print(str(e))
        sys.exit(1)

    stop_spinner()
    if warnings:
        print("\n".join(warnings))
    print(f"✅ {BCOLORS.OKGREEN}Modguard initialized.")
    sys.exit(0)


def main() -> None:
    args, parser = parse_arguments(sys.argv[1:])
    if args.command == "add":
        paths = set(args.path.split(","))
        tags = set(args.tags.split(",")) if args.tags else None
        modguard_add(paths=paths, tags=tags)
        return
    exclude_paths = args.exclude.split(",") if args.exclude else None
    if args.command == "init":
        start_spinner("Initializing...")
        modguard_init(exclude_paths=exclude_paths)
    elif args.command == "check":
        start_spinner("Scanning...")
        modguard_check(exclude_paths=exclude_paths)
    else:
        print("Unrecognized command")
        parser.print_help()
        exit(1)


if __name__ == "__main__":
    main()
