from __future__ import annotations

from pathlib import Path

from tach.filesystem.service import mark_executable, write_file
from tach.hooks import build_pre_commit_hook_content


def install_pre_commit(path: str = ".", project_root: str = "") -> tuple[bool, str]:
    root_path = Path(path)

    git_hooks_dir = root_path / ".git/hooks"
    hook_dst = git_hooks_dir / "pre-commit"

    if not git_hooks_dir.exists():
        return False, f"'{git_hooks_dir}' directory does not exist"

    if hook_dst.exists():
        return False, f"'{hook_dst}' already exists, you'll need to install manually"

    pre_commit_hook_content = build_pre_commit_hook_content(root=project_root)

    write_file(str(hook_dst), pre_commit_hook_content)
    mark_executable(str(hook_dst))
    return True, ""
