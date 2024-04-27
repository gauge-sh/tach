import argparse
import sys
from typing import Optional

from modguard.check import check, ErrorInfo
from modguard import filesystem as fs
from modguard.init import init_project
from modguard.loading import stop_spinner, start_spinner
from modguard.parsing import parse_project_config, build_module_trie
from modguard.show import show
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
        help="Initialize boundaries between top-level modules and write dependencies to "
        "`modguard.yml`",
        description="Initialize boundaries between top-level modules and write dependencies to "
        "`modguard.yml`",
    )
    add_base_arguments(init_parser)
    check_parser = subparsers.add_parser(
        "check",
        prog="modguard check",
        help="Check existing boundaries against your dependencies and module interfaces",
        description="Check existing boundaries against your dependencies and module interfaces",
    )
    add_base_arguments(check_parser)
    show_parser = subparsers.add_parser(
        "show",
        prog="modguard show",
        help="Show your existing boundaries",
        description="Show your existing boundaries",
    )
    add_base_arguments(show_parser)
    show_parser.add_argument(
        "-w",
        "--write",
        required=False,
        dest="write",
        action="store_true",
        default=False,
        help="Write the output to an `interface.yaml` file",
    )
    return parser


def parse_arguments(args: list[str]) -> argparse.Namespace:
    parser = build_parser()
    parsed_args = parser.parse_args(args)

    if not args[0] == "init":
        fs.validate_project_config_path()

    return parsed_args


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
        raise e
        stop_spinner()
        print(str(e))
        sys.exit(1)

    stop_spinner()
    if result:
        print_errors(result)
        sys.exit(1)
    print(f"✅ {BCOLORS.OKGREEN}All modules safely guarded!")
    sys.exit(0)


def modguard_show(
    write_file: bool,
    exclude_paths: Optional[list[str]] = None,
    exclude_hidden_paths: Optional[bool] = True,
):
    try:
        mt = build_module_trie(
            ".", exclude_paths=exclude_paths, exclude_hidden_paths=exclude_hidden_paths
        )
        _, pretty_result = show(mt, write_file=write_file)
    except Exception as e:
        stop_spinner()
        print(str(e))
        sys.exit(1)
    stop_spinner()
    print(pretty_result)
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


def main() -> None:
    args = parse_arguments(sys.argv[1:])
    exclude_paths = args.exclude.split(",") if args.exclude else None
    if args.command == "init":
        start_spinner("Initializing...")
        modguard_init(exclude_paths=exclude_paths)
    elif args.command == "check":
        start_spinner("Scanning...")
        modguard_check(exclude_paths=exclude_paths)
    elif args.command == "show":
        start_spinner("Scanning...")
        modguard_show(write_file=args.write, exclude_paths=exclude_paths)
    else:
        print("Unrecognized command")
        exit(1)


if __name__ == "__main__":
    main()
