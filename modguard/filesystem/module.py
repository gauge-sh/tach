import os
from typing import Optional

import yaml

from modguard.constants import MODULE_FILE_NAME
from modguard.core.config import ModuleConfig


def validate_module_config(root=".") -> Optional[str]:
    file_path = os.path.join(root, f"{MODULE_FILE_NAME}.yml")
    if os.path.exists(file_path):
        return file_path
    file_path = os.path.join(root, f"{MODULE_FILE_NAME}.yaml")
    if os.path.exists(file_path):
        return file_path
    return


def parse_module_config(root=".") -> Optional[ModuleConfig]:
    file_path = validate_module_config(root)
    if file_path:
        with open(file_path, "r") as f:
            results = yaml.safe_load(f)
        return ModuleConfig(**results)
