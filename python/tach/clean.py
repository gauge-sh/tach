from __future__ import annotations

from tach import filesystem as fs


def clean_project(root: str) -> None:
    """
    Remove all tach-related configuration from project root.
    """

    project_config_path = fs.get_project_config_path(root)
    if project_config_path:
        fs.delete_file(project_config_path)

    for _, package_config_path in fs.walk_configured_packages(root=root):
        fs.delete_file(package_config_path)
