from __future__ import annotations

import os

from tach.constants import PACKAGE_FILE_NAME


def get_package_config_path(root: str = ".") -> str | None:
    file_path = os.path.join(root, f"{PACKAGE_FILE_NAME}.yml")
    if os.path.exists(file_path):
        return file_path
    file_path = os.path.join(root, f"{PACKAGE_FILE_NAME}.yaml")
    if os.path.exists(file_path):
        return file_path
    return
