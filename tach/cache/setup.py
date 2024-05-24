from __future__ import annotations

import uuid
from pathlib import Path


def resolve_project_dir() -> Path:
    current_dir = Path.cwd()
    tach_path = current_dir / "tach.yml"
    if not tach_path.exists():
        raise FileNotFoundError()
    return Path(current_dir)


def resolve_dot_tach() -> Path:
    project_dir = resolve_project_dir()

    def _create(path: Path, is_file: bool = False, file_content: str = "") -> None:
        if not path.exists():
            if is_file:
                path.write_text(file_content.strip())
            else:
                path.mkdir()

    # Create .bridge
    tach_path = project_dir / ".tach"
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
