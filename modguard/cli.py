import argparse
import os
from modguard.check import check, ErrorInfo

class BCOLORS:
    HEADER = '\033[95m'
    OKBLUE = '\033[94m'
    OKCYAN = '\033[96m'
    OKGREEN = '\033[92m'
    WARNING = '\033[93m'
    FAIL = '\033[91m'
    ENDC = '\033[0m'
    BOLD = '\033[1m'
    UNDERLINE = '\033[4m'


parser = argparse.ArgumentParser(
    prog="modguard",
    description="Verify module boundaries are correctly implemented.",
    epilog="Make sure modguard is run from the root of your repo that a directory is being specified. For example: `modguard .`",
)

parser.add_argument("path", type=str, help="The path of the root of your project that contains all defined boundaries.")


def execute():
    args: argparse.Namespace = parser.parse_args()
    path = args.path
    if not os.path.isdir(path):
        print(f'{BCOLORS.FAIL} {path} is not a valid directory. Provide the path of the root of your project.')
    result: list[ErrorInfo] = check(path)
    if result:
        sorted_results = sorted(result, key=lambda e: e.location)
        for error in sorted_results:
            print(f"❌ {BCOLORS.FAIL}{error.location}{BCOLORS.WARNING}: {error.message}")
    else:
        print(f"✅ {BCOLORS.OKGREEN}All modules safely guarded!")
