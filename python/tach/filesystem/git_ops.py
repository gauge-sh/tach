from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from tach.errors import TachError, TachSetupError


@dataclass
class GitBranchInfo:
    repo: str
    name: str
    commit: str


def get_current_branch_info(
    project_root: Path, allow_dirty: bool = False
) -> GitBranchInfo:
    # Local import because git-python takes ~80ms to load
    from git import InvalidGitRepositoryError, NoSuchPathError, Repo

    try:
        repo = Repo(project_root, search_parent_directories=True)
    except (InvalidGitRepositoryError, NoSuchPathError):
        raise TachSetupError("The project does not appear to be a git repository!")

    # Fail if the branch is not clean
    if not allow_dirty and repo.is_dirty():
        raise TachError(
            "The current branch is not clean, please commit your changes before running this command!"
        )

    try:
        # TODO: support slashes or org names
        repo_name = repo.remotes.origin.url.split("/")[-1].replace(".git", "")
        branch = repo.active_branch.name
        commit = repo.head.commit.hexsha
    except Exception as e:
        raise TachError(f"Failed to determine current branch information!\nError: {e}")

    return GitBranchInfo(repo=repo_name, name=branch, commit=commit)


def get_changed_files(
    project_root: Path, head: str = "", base: str = "main"
) -> list[Path]:
    # Local import because git-python takes ~80ms to load
    from git import GitCommandError, InvalidGitRepositoryError, NoSuchPathError, Repo

    try:
        repo = Repo(project_root, search_parent_directories=True)
    except (InvalidGitRepositoryError, NoSuchPathError):
        raise TachSetupError(
            "The project does not appear to be a git repository, cannot determine changed files!"
        )

    try:
        if head:
            diff: str = repo.git.diff("--name-status", head, base)
        else:
            # If head is not provided, we can diff against 'base' from the current filesystem
            diff: str = repo.git.diff("--name-status", base)
    except GitCommandError:
        head_display = f"'{head}'" if head else "current filesystem"
        raise TachError(f"Failed to check diff between '{base}' and {head_display}!")

    diff_lines = diff.splitlines()
    changed_files: set[str] = set()

    for line in diff_lines:
        _, *files = line.split("\t")
        changed_files.update(files)

    if not head:
        # If we are using the current filesystem, there may be relevant changes in untracked files
        untracked_files: str = repo.git.ls_files("--others", "--exclude-standard")
        changed_files.update(untracked_files.splitlines())

    # return list of unique Paths
    git_root: str = repo.git.rev_parse("--show-toplevel")
    return [(Path(git_root) / filepath).resolve() for filepath in changed_files]


__all__ = ["get_changed_files", "get_current_branch_info"]
