import os
import sys

from modguard.colors import BCOLORS
from modguard.constants import CONFIG_FILE_NAME


def print_no_modguard_yml() -> None:
    print(
        f"{BCOLORS.FAIL} {CONFIG_FILE_NAME}.(yml|yaml) not found in {os.getcwd()}",
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
