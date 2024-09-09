from __future__ import annotations

from typing import TYPE_CHECKING

from tach.filesystem.service import mark_executable, write_file
from tach.hooks import build_pre_commit_hook_content

if TYPE_CHECKING:
    from pathlib import Path


def install_pre_commit(project_root: Path) -> tuple[bool, str]:
    git_hooks_dir = project_root / ".git" / "hooks"
    hook_dst = git_hooks_dir / "pre-commit"

    if not git_hooks_dir.exists():
        return False, f"'{git_hooks_dir}' directory does not exist"

    if hook_dst.exists():
        return False, f"'{hook_dst}' already exists, you'll need to install manually"

    pre_commit_hook_content = build_pre_commit_hook_content()

    write_file(hook_dst, pre_commit_hook_content, root=project_root)
    mark_executable(hook_dst)
    return True, ""
