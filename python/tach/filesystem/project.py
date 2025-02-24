from __future__ import annotations

from pathlib import Path

from tach.constants import CONFIG_FILE_NAME


def build_project_config_path(root: Path, file_name: str = CONFIG_FILE_NAME) -> Path:
    return root / f"{file_name}.toml"


def get_project_config_path(
    root: Path, *, file_name: str = CONFIG_FILE_NAME
) -> Path | None:
    file_path = build_project_config_path(root, file_name)
    if file_path.exists():
        return file_path
    return None


def get_pyproject_config_path(root: Path) -> Path | None:
    file_path = root / "pyproject.toml"
    if file_path.exists() and "tool.tach" in file_path.read_text():
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


def has_project_config(root: Path) -> bool:
    return (
        get_project_config_path(root) is not None
        or get_pyproject_config_path(root) is not None
    )


def find_project_config_root() -> Path | None:
    cwd = Path.cwd()

    if has_project_config(cwd):
        return cwd

    # Iterate upwards, looking for project config
    for parent in cwd.parents:
        if has_project_config(parent):
            return parent

    return None
