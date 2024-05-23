from __future__ import annotations

import os
import uuid
from pathlib import Path


def resolve_project_dir() -> Path:
    current_dir = os.getcwd()
    tach_path = Path(os.path.join(current_dir, "tach.yml"))
    if not tach_path.exists():
        raise FileNotFoundError()
    return Path(current_dir)


def resolve_dot_tach() -> Path:
    project_dir = resolve_project_dir()

    def _create(path: str, is_file: bool = False, file_content: str = "") -> None:
        if not os.path.exists(path):
            if is_file:
                with open(path, "w") as f:
                    f.write(file_content.strip())
            else:
                os.makedirs(path)

    # Create .bridge
    tach_path = os.path.join(project_dir, ".tach")
    _create(tach_path)
    # Create info

    info_path = os.path.join(tach_path, "tach.info")
    _create(info_path, is_file=True, file_content=str(uuid.uuid4()))
    # Create .gitignore
    gitignore_content = """
# This folder is for tach. Do not edit.

# gitignore all content, including this .gitignore
*
    """
    gitignore_path = os.path.join(tach_path, ".gitignore")
    _create(gitignore_path, is_file=True, file_content=gitignore_content)
    return Path(tach_path)
