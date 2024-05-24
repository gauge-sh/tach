from __future__ import annotations

import uuid
from pathlib import Path
from typing import Optional

from tach.cache.setup import resolve_dot_tach
from tach.filesystem import find_project_config_root


def get_uid() -> Optional[uuid.UUID]:
    project_root = find_project_config_root(str(Path.cwd()))
    if project_root is None:
        return
    project_path = Path(project_root)
    if not (project_path / ".tach" / "tach.info").exists():
        resolve_dot_tach()
    with open(project_path / ".tach" / "tach.info") as f:
        uid = uuid.UUID(f.read().strip())
    return uid
