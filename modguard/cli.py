import argparse
import os
import sys

from modguard.check import check, ErrorInfo
from modguard.init import init_project
from modguard.show import show
from modguard.parsing.boundary import build_boundary_trie


class BCOLORS:
    HEADER = "\033[95m"
    OKBLUE = "\033[94m"
    OKCYAN = "\033[96m"
    OKGREEN = "\033[92m"
    WARNING = "\033[93m"
    FAIL = "\033[91m"
    ENDC = "\033[0m"
    BOLD = "\033[1m"
    UNDERLINE = "\033[4m"


def print_errors(error_list: list[ErrorInfo]) -> None:
    sorted_results = sorted(error_list, key=lambda e: e.location)
    for error in sorted_results:
        print(
            f"❌ {BCOLORS.FAIL}{error.location}{BCOLORS.WARNING}: {error.message}",
            file=sys.stderr,
        )


def print_invalid_path(path: str) -> None:
    print(
        f"{BCOLORS.FAIL} {path} is not a valid directory!! Provide the path of the root of your project.",
        file=sys.stderr,
    )


def print_invalid_exclude(path: str) -> None:
    print(
        f"{BCOLORS.FAIL} {path} is not a valid dir or file. Make sure the exclude list is comma separated and valid.",
        file=sys.stderr,
    )


def parse_base_arguments(args: list[str]) -> argparse.Namespace:
    base_parser = argparse.ArgumentParser(
        prog="modguard",
        add_help=True,
        epilog="Make sure modguard is run from the root of your repo that a directory is being specified. For example: `modguard .`",
    )
    base_parser.add_argument(
        "path",
        type=str,
        help="The path of the root of your project that contains all defined boundaries.",
    )
    base_parser.add_argument(
        "-e",
        "--exclude",
        required=False,
        type=str,
        metavar="file_or_path,...",
        help="Comma separated path list to exclude. tests/,ci/,etc.",
    )
    return base_parser.parse_args(args)


def parse_init_arguments(args: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="modguard init",
        description="Initialize boundaries in a repository with modguard",
    )
    parser.add_argument(
        "path",
        type=str,
        help="The path of the Python project in which boundaries should be initialized.",
    )
    parser.add_argument(
        "-e",
        "--exclude",
        required=False,
        type=str,
        metavar="file_or_path,...",
        help="Comma separated path list to exclude. tests/,ci/,etc.",
    )

    return parser.parse_args(args)


def parse_show_arguments(args: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="modguard show",
        description="Show your exisiting boundaries in modguard",
    )
    parser.add_argument(
        "path",
        type=str,
        help="The path of the Python project in which boundaries should be initialized.",
    )
    parser.add_argument(
        "-e",
        "--exclude",
        required=False,
        type=str,
        metavar="file_or_path,...",
        help="Comma separated path list to exclude. tests/,ci/,etc.",
    )
    parser.add_argument(
        "-w",
        "--write",
        required=False,
        dest="write",
        action='store_true',
        default=False,
        help="Include to write the output to a `modguard.yaml` file"
    )
    return parser.parse_args(args)


def handle_shared_arguments(args: argparse.Namespace):
    path = args.path
    if not os.path.isdir(path):
        print_invalid_path(path)
        sys.exit(1)
    exclude_paths = args.exclude
    if exclude_paths:
        has_error = False
        for exclude_path in exclude_paths.split(","):
            if (
                exclude_path
                and not os.path.isdir(exclude_path)
                and not os.path.isfile(exclude_path)
            ):
                has_error = True
                print_invalid_exclude(exclude_path)
        if has_error:
            sys.exit(1)

    return argparse.Namespace(
        path=path, exclude_paths=exclude_paths.split(",") if exclude_paths else None
    )


def modguard(args: argparse.Namespace):
    shared_args = handle_shared_arguments(args)
    try:
        result: list[ErrorInfo] = check(
            shared_args.path, exclude_paths=shared_args.exclude_paths
        )
    except Exception as e:
        print(str(e))
        sys.exit(1)

    if result:
        print_errors(result)
        sys.exit(1)
    print(f"✅ {BCOLORS.OKGREEN}All modules safely guarded!")
    sys.exit(0)


def modguard_show(args: argparse.Namespace):
    shared_args = handle_shared_arguments(args)
    try:
        bt = build_boundary_trie(shared_args.path)
        show(bt, write_file=args.write)
    except Exception as e:
        print(str(e))
        sys.exit(1)
    sys.exit(0)


def modguard_init(args: argparse.Namespace):
    shared_args = handle_shared_arguments(args)
    try:
        init_project(shared_args.path, exclude_paths=shared_args.exclude_paths)
    except Exception as e:
        print(str(e))
        sys.exit(1)

    print(f"✅ {BCOLORS.OKGREEN}Modguard initialized.")
    sys.exit(0)


def main() -> None:
    if len(sys.argv) > 1 and sys.argv[1] == "init":
        modguard_init(parse_init_arguments(sys.argv[2:]))
    if len(sys.argv) > 1 and sys.argv[1] == "show":
        modguard_show(parse_show_arguments(sys.argv[2:]))
    else:
        modguard(parse_base_arguments(sys.argv[1:]))


if __name__ == "__main__":
    main()
