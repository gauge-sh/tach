from __future__ import annotations

import uuid

from tach.cache.setup import resolve_dot_tach, resolve_project_dir


def get_uid() -> uuid.UUID:
    project_root = resolve_project_dir()
    if not (project_root / ".tach" / "tach.info").exists():
        resolve_dot_tach()
    with open(project_root / ".tach" / "tach.info") as f:
        uid = uuid.UUID(f.read().strip())
    return uid
