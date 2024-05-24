from __future__ import annotations

import uuid
from pathlib import Path
from typing import Optional

from tach.filesystem import find_project_config_root


def resolve_dot_tach() -> Optional[Path]:
    project_dir = find_project_config_root(str(Path.cwd()))
    if project_dir is None:
        return
    project_path = Path(project_dir)

    def _create(path: Path, is_file: bool = False, file_content: str = "") -> None:
        if not path.exists():
            if is_file:
                path.write_text(file_content.strip())
            else:
                path.mkdir()

    # Create .tach
    tach_path = project_path / ".tach"
    _create(tach_path)
    # Create info
    info_path = tach_path / "tach.info"
    _create(info_path, is_file=True, file_content=str(uuid.uuid4()))
    # Create .gitignore
    gitignore_content = """
# This folder is for tach. Do not edit.

# gitignore all content, including this .gitignore
*
    """
    gitignore_path = tach_path / ".gitignore"
    _create(gitignore_path, is_file=True, file_content=gitignore_content)
    return Path(tach_path)
