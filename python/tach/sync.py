from __future__ import annotations

from typing import TYPE_CHECKING

from tach import errors
from tach.extension import (
    sync_project as sync_project_ext,
)
from tach.filesystem import get_project_config_path

if TYPE_CHECKING:
    from pathlib import Path

    from tach.extension import ProjectConfig


def sync_project(
    project_root: Path,
    project_config: ProjectConfig,
    exclude_paths: list[str],
    add: bool = False,
) -> None:
    config_path = get_project_config_path(project_root)
    if config_path is None:
        raise errors.TachError(
            "Unexpected error. Could not find configuration file during 'sync'."
        )

    sync_project_ext(
        project_root=project_root,
        project_config=project_config,
        exclude_paths=exclude_paths,
        add=add,
    )


__all__ = ["sync_project"]
