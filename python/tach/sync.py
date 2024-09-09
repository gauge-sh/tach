from __future__ import annotations

from typing import TYPE_CHECKING

import tomli
import tomli_w

from tach import errors
from tach import filesystem as fs
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

    config_toml_content = sync_project_ext(
        project_root=project_root,
        project_config=project_config,
        exclude_paths=exclude_paths,
        add=add,
    )
    # Format the content, TODO: should'nt be handled here
    config_toml_content = tomli_w.dumps(tomli.loads(config_toml_content))
    fs.write_file(config_path, config_toml_content, root=project_root)


__all__ = ["sync_project"]
