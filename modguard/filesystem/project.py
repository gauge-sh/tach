import os
import sys
from typing import Optional

from modguard.cli import print_no_modguard_yml
from modguard.constants import CONFIG_FILE_NAME, MODULE_FILE_NAME
from modguard.core.config import ProjectConfig, ModuleConfig
import yaml


def validate_project_config_path(root=".") -> str:
    file_path = os.path.join(root, f"{CONFIG_FILE_NAME}.yml")
    if os.path.exists(file_path):
        return file_path
    file_path = os.path.join(root, f"{CONFIG_FILE_NAME}.yaml")
    if os.path.exists(file_path):
        return file_path
    print_no_modguard_yml()
    sys.exit(1)


def parse_project_config(root=".") -> ProjectConfig:
    file_path = validate_project_config_path(root)
    with open(file_path, "r") as f:
        results = yaml.safe_load(f)
    return ProjectConfig(**results)


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
