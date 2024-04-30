from typing import Optional

import yaml

from tach.core import ProjectConfig, PackageConfig
from tach import filesystem as fs


def parse_project_config(root: str = ".") -> ProjectConfig:
    file_path = fs.validate_project_config_path(root)
    with open(file_path, "r") as f:
        result = yaml.safe_load(f)
        if not result or not isinstance(result, dict):
            raise ValueError(f"Empty or invalid project config file: {file_path}")
    # We want to error on type issues here for now
    project_config = ProjectConfig(**result)  # type: ignore
    return project_config


def parse_package_config(root: str = ".") -> Optional[PackageConfig]:
    file_path = fs.validate_package_config(root)
    if file_path:
        with open(file_path, "r") as f:
            result = yaml.safe_load(f)
            if not result or not isinstance(result, dict):
                raise ValueError(f"Empty or invalid package config file: {file_path}")
        # We want to error on type issues here for now
        return PackageConfig(**result)  # type: ignore
