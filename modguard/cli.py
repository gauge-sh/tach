import argparse
from modguard.check import check

parser = argparse.ArgumentParser(
    prog="modguard",
    description="Verify module boundaries are correctly implemented.",
    epilog="Make sure modguard is run from the root of your repo that a directory is being specified. For example, `modguard .`",
)

parser.add_argument("path")


def execute():
    args = parser.parse_args()
    print(check(args.path))
