import argparse
import os
import sys
from typing import Optional

from modguard.check import check, ErrorInfo
from modguard.init import init_project
from modguard.loading import stop_spinner, start_spinner
from modguard.show import show
from modguard.parsing.boundary import build_boundary_trie
from modguard.colors import BCOLORS


def print_errors(error_list: list[ErrorInfo]) -> None:
    sorted_results = sorted(error_list, key=lambda e: e.location)
    for error in sorted_results:
        print(
            f"❌ {BCOLORS.FAIL}{error.location}{BCOLORS.WARNING}: {error.message}",
            file=sys.stderr,
        )


def print_invalid_path(path: str) -> None:
    print(
        f"{BCOLORS.FAIL} {path} is not a valid directory! Provide the path of the root of your project.",
        file=sys.stderr,
    )


def print_invalid_exclude(path: str) -> None:
    print(
        f"{BCOLORS.FAIL} {path} is not a valid dir or file. Make sure the exclude list is comma separated and valid.",
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
    parser.add_argument(
        "path",
        type=str,
        help="The path of the root of your Python project.",
    )


def parse_arguments(args: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="modguard",
        add_help=True,
        epilog="Make sure modguard is run from the root of your Python project and that a directory is being specified. For example: `modguard check .`",
    )
    subparsers = parser.add_subparsers(title="commands", dest="command")
    init_parser = subparsers.add_parser(
        "init",
        prog="modguard init",
        help="Initialize boundaries and mark imported members as public",
        description="Initialize boundaries with modguard",
    )
    add_base_arguments(init_parser)
    check_parser = subparsers.add_parser(
        "check",
        prog="modguard check",
        help="Check existing boundaries against marked members",
        description="Check boundaries with modguard",
    )
    add_base_arguments(check_parser)
    show_parser = subparsers.add_parser(
        "show",
        prog="modguard show",
        help="Show your existing boundaries and optionally write to yaml",
        description="Show your existing boundaries in modguard",
    )
    add_base_arguments(show_parser)
    show_parser.add_argument(
        "-w",
        "--write",
        required=False,
        dest="write",
        action="store_true",
        default=False,
        help="Write the output to a `modguard.yaml` file",
    )
    parsed_args = parser.parse_args(args)
    path = parsed_args.path
    if not os.path.isdir(path):
        print_invalid_path(path)
        sys.exit(1)
    exclude_paths = parsed_args.exclude
    if exclude_paths:
        exclude_paths = exclude_paths.split(",")
        has_error = False
        for exclude_path in exclude_paths:
            if (
                exclude_path
                and not os.path.isdir(exclude_path)
                and not os.path.isfile(exclude_path)
            ):
                has_error = True
                print_invalid_exclude(exclude_path)
        if has_error:
            sys.exit(1)
    return parsed_args


def modguard_check(args: argparse.Namespace, exclude_paths: Optional[list[str]] = None):
    try:
        result: list[ErrorInfo] = check(args.path, exclude_paths=exclude_paths)
    except Exception as e:
        stop_spinner()
        print(str(e))
        sys.exit(1)

    stop_spinner()
    if result:
        print_errors(result)
        sys.exit(1)
    print(f"✅ {BCOLORS.OKGREEN}All modules safely guarded!")
    sys.exit(0)


def modguard_show(args: argparse.Namespace, exclude_paths: Optional[list[str]] = None):
    try:
        bt = build_boundary_trie(args.path, exclude_paths=exclude_paths)
        _, pretty_result = show(bt, write_file=args.write)
    except Exception as e:
        stop_spinner()
        print(str(e))
        sys.exit(1)
    stop_spinner()
    print(pretty_result)
    sys.exit(0)


def modguard_init(args: argparse.Namespace, exclude_paths: Optional[list[str]] = None):
    try:
        warnings = init_project(args.path, exclude_paths=exclude_paths)
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
        modguard_init(args, exclude_paths)
    elif args.command == "check":
        start_spinner("Scanning...")
        modguard_check(args, exclude_paths)
    elif args.command == "show":
        start_spinner("Scanning...")
        modguard_show(args, exclude_paths)
    else:
        print("Unrecognized command")
        exit(1)


if __name__ == "__main__":
    main()
