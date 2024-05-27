from __future__ import annotations

import os
from pathlib import Path
from typing import Optional

from tach.constants import CONFIG_FILE_NAME


# TODO convert all str paths to pathlib.Path
def get_project_config_path(root: str = ".") -> str:
    file_path = os.path.join(root, f"{CONFIG_FILE_NAME}.yml")
    if os.path.exists(file_path):
        return file_path
    file_path = os.path.join(root, f"{CONFIG_FILE_NAME}.yaml")
    if os.path.exists(file_path):
        return file_path
    return ""


def find_project_config_root(path: str) -> Optional[str]:
    path = os.path.abspath(path)
    if os.path.isdir(path):
        if get_project_config_path(path):
            return path
    path_obj = Path(path)
    # Iterate upwards, looking for project config
    for parent in path_obj.parents:
        if get_project_config_path(str(parent)):
            return str(parent)
