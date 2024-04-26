import os
from typing import Optional

from modguard.constants import MODULE_FILE_NAME


def validate_module_config(root: str = ".") -> Optional[str]:
    file_path = os.path.join(root, f"{MODULE_FILE_NAME}.yml")
    if os.path.exists(file_path):
        return file_path
    file_path = os.path.join(root, f"{MODULE_FILE_NAME}.yaml")
    if os.path.exists(file_path):
        return file_path
    return
