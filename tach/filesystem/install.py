import shutil
from importlib import resources
from pathlib import Path

from tach.constants import PACKAGE_NAME
from tach.filesystem.service import mark_executable


def install_pre_commit(path: str = ".") -> tuple[bool, str]:
    root_path = Path(path)

    try:
        hook_src = resources.files(PACKAGE_NAME) / "hooks/pre-commit"
    except ModuleNotFoundError:
        return False, f"{PACKAGE_NAME} could not be found, verify your installation"

    if not hook_src.is_file():
        return False, (
            f"pre-commit hook not found in {PACKAGE_NAME} installation. "
            "This is likely a bug in the current version"
        )

    git_hooks_dir = root_path / ".git/hooks"
    hook_dst = git_hooks_dir / "pre-commit"

    if not git_hooks_dir.exists():
        return False, f"'{git_hooks_dir}' directory does not exist"

    if hook_dst.exists():
        return False, f"'{hook_dst}' already exists, you'll need to install manually"

    shutil.copy(str(hook_src), str(hook_dst))
    mark_executable(str(hook_dst))
    return True, ""
