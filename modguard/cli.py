import argparse
from modguard.check import check, ErrorInfo

parser = argparse.ArgumentParser(
    prog="modguard",
    description="Verify module boundaries are correctly implemented.",
    epilog="Make sure modguard is run from the root of your repo that a directory is being specified. For example: `modguard .`",
)

parser.add_argument("path", type=str, help="The path of the root of your project that contains all defined boundaries.")


def execute():
    args: argparse.Namespace = parser.parse_args()
    path = args.path
    result: list[ErrorInfo] = check(path)
    if result:
        for error in result:
            print("❌", error.location, ":", error.message)
    else:
        print("✅ all modules safely guarded!")
