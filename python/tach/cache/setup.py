from __future__ import annotations

import uuid
from pathlib import Path

from tach import __version__
from tach.filesystem import find_project_config_root


def resolve_dot_tach() -> Path | None:
    project_path = find_project_config_root()
    if project_path is None:
        return

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
    # Create version
    version_path = tach_path / ".latest-version"
    _create(version_path, is_file=True, file_content=__version__)
    return Path(tach_path)
