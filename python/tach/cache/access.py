from __future__ import annotations

import uuid

from tach.cache.setup import resolve_dot_tach
from tach.filesystem import find_project_config_root

# TODO: don't call find_project_config_root separately in each function, pass as argument


def get_uid() -> uuid.UUID | None:
    project_path = find_project_config_root()
    if project_path is None:
        return
    info_path = project_path / ".tach" / "tach.info"
    if not info_path.exists():
        resolve_dot_tach()
    contents = info_path.read_text().strip()
    uid = uuid.UUID(contents)
    return uid


def get_latest_version() -> str | None:
    project_path = find_project_config_root()
    if project_path is None:
        return
    latest_version_path = project_path / ".tach" / ".latest-version"
    if not latest_version_path.exists():
        return
    version = latest_version_path.read_text().strip()
    return version
