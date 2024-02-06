import argparse
import os
import sys
from modguard.check import check, ErrorInfo


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


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="modguard",
        description="Verify module boundaries are correctly implemented.",
        epilog="Make sure modguard is run from the root of your repo that a directory is being specified. For example: `modguard .`",
    )

    parser.add_argument(
        "path",
        type=str,
        help="The path of the root of your project that contains all defined boundaries.",
    )
    parser.add_argument(
        "-e",
        "--exclude",
        required=False,
        type=str,
        metavar="file_or_path,...",
        help="Comma separated path list to exclude. tests/,ci/,etc.",
    )
    return parser


def main(args: argparse.Namespace):
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
    result: list[ErrorInfo] = check(path, exclude_paths=exclude_paths)
    if result:
        print_errors(result)
        sys.exit(1)
    print(f"✅ {BCOLORS.OKGREEN}All modules safely guarded!")
    sys.exit(0)


def modguard() -> None:
    parser = build_parser()
    args = parser.parse_args()
    main(args)
