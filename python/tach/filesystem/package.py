from __future__ import annotations

import os
from typing import Optional

from tach.constants import PACKAGE_FILE_NAME


def validate_package_config(root: str = ".") -> Optional[str]:
    file_path = os.path.join(root, f"{PACKAGE_FILE_NAME}.yml")
    if os.path.exists(file_path):
        return file_path
    file_path = os.path.join(root, f"{PACKAGE_FILE_NAME}.yaml")
    if os.path.exists(file_path):
        return file_path
    return
