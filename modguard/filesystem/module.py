import os
from typing import Optional

import yaml

from modguard.constants import MODULE_FILE_NAME
from modguard.core.config import ModuleConfig


def validate_module_config(root: str = ".") -> Optional[str]:
    file_path = os.path.join(root, f"{MODULE_FILE_NAME}.yml")
    if os.path.exists(file_path):
        return file_path
    file_path = os.path.join(root, f"{MODULE_FILE_NAME}.yaml")
    if os.path.exists(file_path):
        return file_path
    return


def parse_module_config(root: str = ".") -> Optional[ModuleConfig]:
    file_path = validate_module_config(root)
    if file_path:
        with open(file_path, "r") as f:
            result = yaml.safe_load(f)
            if not result or not isinstance(result, dict):
                raise ValueError(f"Empty or invalid module config file: {file_path}")
        return ModuleConfig(**result)  # type: ignore we want to error here for now
