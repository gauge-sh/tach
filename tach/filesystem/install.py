import shutil
from pathlib import Path
from tach.filesystem.service import mark_executable


def install_pre_commit(path: str = ".") -> tuple[bool, str]:
    root_path = Path(path)
    hook_src = root_path / "hooks/pre-commit"
    git_hooks_dir = root_path / ".git/hooks"
    hook_dst = git_hooks_dir / "pre-commit"

    if not git_hooks_dir.exists():
        return False, f"'{git_hooks_dir}' directory does not exist"

    if hook_dst.exists():
        return False, f"'{hook_dst}' already exists, you'll need to install manually"

    shutil.copy(str(hook_src), str(hook_dst))
    mark_executable(str(hook_dst))
    return True, ""
