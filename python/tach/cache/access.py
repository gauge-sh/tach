from __future__ import annotations

import uuid

from tach.cache.setup import get_project_path, resolve_dot_tach


def get_uid() -> uuid.UUID | None:
    project_path = get_project_path()
    if project_path is None:
        return
    if not (project_path / ".tach" / "tach.info").exists():
        resolve_dot_tach()
    with open(project_path / ".tach" / "tach.info") as f:
        uid = uuid.UUID(f.read().strip())
    return uid


def get_latest_version() -> str | None:
    project_path = get_project_path()
    if project_path is None:
        return
    if not (project_path / ".tach" / ".latest-version").exists():
        resolve_dot_tach()
        update_latest_version()
    with open(project_path / ".tach" / ".latest-version") as f:
        version = f.read().strip()
    return version


def update_latest_version() -> None:
    # TODO make api request to https://pypi.org/pypi/tach/json
    project_path = get_project_path()
    if project_path is None:
        return
    (project_path / ".tach" / ".latest-version").write_text("0.5.2")
