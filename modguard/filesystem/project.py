import os
import sys

from modguard.colors import BCOLORS
from modguard.constants import CONFIG_FILE_NAME


def print_no_modguard_yml() -> None:
    print(
        f"{BCOLORS.FAIL} {CONFIG_FILE_NAME}.(yml|yaml) not found in {os.getcwd()}",
        file=sys.stderr,
    )


def print_invalid_exclude(path: str) -> None:
    print(
        f"{BCOLORS.FAIL} {path} is not a valid dir or file. "
        f"Make sure the exclude list is comma separated and valid.",
        file=sys.stderr,
    )


def validate_project_config_path(root: str = ".") -> str:
    file_path = os.path.join(root, f"{CONFIG_FILE_NAME}.yml")
    if os.path.exists(file_path):
        return file_path
    file_path = os.path.join(root, f"{CONFIG_FILE_NAME}.yaml")
    if os.path.exists(file_path):
        return file_path
    print_no_modguard_yml()
    sys.exit(1)


def validate_exclude_paths(exclude_paths: list[str]) -> None:
    has_error = False
    for exclude_path in exclude_paths:
        if (
            exclude_path
            and not os.path.isdir(exclude_path)
            and not os.path.isfile(exclude_path)
        ):
            print_invalid_exclude(exclude_path)
            has_error = True
    if has_error:
        sys.exit(1)
