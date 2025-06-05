from __future__ import annotations

import uuid
from os import getenv, name as os_name
from pathlib import Path
from re import match, compile

from tach import __version__


def get_cache_path(project_root: Path) -> Path:
    env_value = getenv("TACH_CACHE_DIR")

    if env_value is None:
        return project_root / ".tach"

    path_is_not_absolute_re = compile(r'^(?![a-zA-Z]:[\\/]|\\\\|/|~[/\\])') if os_name == 'nt' else compile(r'^[^/~]')
    if match(path_is_not_absolute_re, env_value):
        return project_root / env_value

    return Path(env_value)


def resolve_cache_path(project_root: Path) -> Path | None:
    def _create(path: Path, is_file: bool = False, file_content: str = "") -> None:
        if not path.exists():
            if is_file:
                path.write_text(file_content.strip())
            else:
                path.mkdir()

    # Create cache dir
    cache_dir = get_cache_path(project_root)
    _create(cache_dir)
    # Create info
    info_path = cache_dir / "tach.info"
    _create(info_path, is_file=True, file_content=str(uuid.uuid4()))
    # Create .gitignore
    gitignore_content = """
# This folder is for tach. Do not edit.

# gitignore all content, including this .gitignore
*
    """
    gitignore_path = cache_dir / ".gitignore"
    _create(gitignore_path, is_file=True, file_content=gitignore_content)
    # Create version
    version_path = cache_dir / ".latest-version"
    _create(version_path, is_file=True, file_content=__version__)
    return Path(cache_dir)
