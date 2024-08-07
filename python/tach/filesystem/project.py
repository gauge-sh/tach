from __future__ import annotations

from pathlib import Path

from tach.constants import CONFIG_FILE_NAME


def build_project_config_path(root: Path | None = None) -> Path:
    root = root or Path.cwd()
    return root / f"{CONFIG_FILE_NAME}.toml"


def get_project_config_path(root: Path | None = None) -> Path | None:
    file_path = build_project_config_path(root)
    if file_path.exists():
        return file_path
    return None


def get_deprecated_project_config_path(root: Path | None = None) -> Path | None:
    root = root or Path.cwd()
    file_path = root / f"{CONFIG_FILE_NAME}.yaml"
    if file_path.exists():
        return file_path
    file_path = root / f"{CONFIG_FILE_NAME}.yml"
    if file_path.exists():
        return file_path
    return None


def find_project_config_root() -> Path | None:
    cwd = Path.cwd()

    if get_project_config_path(cwd) is not None:
        return cwd

    # Iterate upwards, looking for project config
    for parent in cwd.parents:
        if get_project_config_path(parent):
            return parent

    return None
