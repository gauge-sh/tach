import os
import ast
import threading
from collections import defaultdict
from dataclasses import dataclass
from functools import lru_cache
from pathlib import Path
from typing import Optional, Generator
from modguard.errors import ModguardParseError


@dataclass
class FileInfo:
    path: str
    content: Optional[str] = None
    canonical_path: Optional[str] = None
    ast: Optional["ast.AST"] = None


# Thread-local file cache to avoid going to disk as much as possible
thread_local = threading.local()
# Cannot type-hint non-self attributes (https://github.com/python/mypy/issues/2388)
# cwd: str
thread_local.cwd = os.getcwd()
# file_caches_by_cwd: defaultdict[str, dict[str, FileInfo]]
thread_local.file_caches_by_cwd = defaultdict(dict)


def get_cwd() -> str:
    # Use a cached cwd to avoid system calls
    if not hasattr(thread_local, "cwd"):
        thread_local.cwd = os.getcwd()
    return thread_local.cwd


def chdir(path: str):
    # When using chdir, update the cached version
    os.chdir(path)
    thread_local.cwd = os.getcwd()


def _get_file_cache() -> dict[str, FileInfo]:
    if not hasattr(thread_local, "file_caches_by_cwd"):
        thread_local.file_caches_by_cwd = defaultdict(dict)
    file_caches_by_cwd: defaultdict[
        str, dict[str, FileInfo]
    ] = thread_local.file_caches_by_cwd  # type: ignore
    return file_caches_by_cwd[get_cwd()]


def _file_cache_key(path: str) -> str:
    return f"{get_cwd()}:::{path}"


def _cached_file(path: str) -> Optional[FileInfo]:
    return _get_file_cache().get(_file_cache_key(path))


def _set_cached_file(path: str, file_info: FileInfo):
    _get_file_cache()[_file_cache_key(path)] = file_info


def canonical(path: str) -> str:
    cached_file = _cached_file(path)
    if cached_file and cached_file.canonical_path:
        return cached_file.canonical_path

    result = os.path.relpath(os.path.realpath(path), start=get_cwd())

    if cached_file:
        cached_file.canonical_path = result
    else:
        _set_cached_file(path, FileInfo(path=path, canonical_path=result))

    return result


def read_file(path: str) -> str:
    cached_file = _cached_file(path)
    if cached_file and cached_file.content:
        return cached_file.content

    with open(path, "r") as f:
        content = f.read()

    if cached_file:
        cached_file.content = content
    else:
        _set_cached_file(path, FileInfo(path=path, content=content))

    return content


def write_file(path: str, content: str):
    with open(path, "w") as f:
        f.write(content)

    cached_file = _cached_file(path)
    if cached_file:
        cached_file.content = content
        cached_file.ast = None
    else:
        _set_cached_file(path, FileInfo(path=path, content=content))


def parse_ast(path: str) -> ast.AST:
    cached_file = _cached_file(path)
    if cached_file and cached_file.ast:
        return cached_file.ast

    if cached_file and cached_file.content:
        content = cached_file.content
        try:
            ast_result = ast.parse(cached_file.content)
        except SyntaxError as e:
            raise ModguardParseError(f"Syntax error in {path}: {e}")
    else:
        with open(path, "r") as f:
            content = f.read()
        try:
            ast_result = ast.parse(content)
        except SyntaxError as e:
            raise ModguardParseError(f"Syntax error in {path}: {e}")

    if cached_file:
        cached_file.content = content
        cached_file.ast = ast_result
    else:
        _set_cached_file(path, FileInfo(path=path, content=content, ast=ast_result))

    return ast_result


def walk_pyfiles(
    root: str, exclude_paths: Optional[list[str]] = None
) -> Generator[str, None, None]:
    for dirpath, _, filenames in os.walk(root):
        dirpath = canonical(dirpath)
        for filename in filenames:
            file_path = os.path.join(dirpath, filename)
            if exclude_paths is not None and any(
                file_path.startswith(exclude_path) for exclude_path in exclude_paths
            ):
                # Treat excluded paths as invisible
                continue
            if filename.endswith(".py"):
                yield file_path


def walk_pypackages(
    root: str, exclude_paths: Optional[list[str]] = None
) -> Generator[str, None, None]:
    for filepath in walk_pyfiles(root, exclude_paths=exclude_paths):
        init_file_ending = f"{os.path.sep}__init__.py"
        if filepath.endswith(init_file_ending):
            yield filepath[: -len(init_file_ending)]


@lru_cache(maxsize=None)
def file_to_module_path(file_path: str) -> str:
    # Assuming that the file_path has been 'canonicalized' and does not traverse multiple directories
    file_path = file_path.lstrip("./")
    if file_path == ".":
        return ""

    module_path = file_path.replace(os.sep, ".")

    if module_path.endswith(".py"):
        module_path = module_path[:-3]
    if module_path.endswith(".__init__"):
        module_path = module_path[:-9]
    if module_path == "__init__":
        return ""

    return module_path


def path_exists_case_sensitive(p: Path) -> bool:
    if not p.exists():
        return False

    while True:
        if p == p.parent:
            return True
        # If string representation of path is not in parent directory, return False
        if str(p) not in map(str, p.parent.iterdir()):
            return False
        p = p.parent


def module_to_file_path(
    mod_path: str, find_package_init: bool = False
) -> tuple[str, str]:
    # Assumes that the mod_path is correctly formatted and refers to an actual module
    fs_path = mod_path.replace(".", os.path.sep)

    # mod_path may refer to a package
    if path_exists_case_sensitive(Path(fs_path)):
        return (
            os.path.join(fs_path, "__init__.py") if find_package_init else fs_path
        ), ""

    # mod_path may refer to a file module
    file_path = fs_path + ".py"
    if path_exists_case_sensitive(Path(file_path)):
        return file_path, ""

    # mod_path may refer to a member within a file module
    last_sep_index = fs_path.rfind(os.path.sep)
    file_path = fs_path[:last_sep_index] + ".py"
    if path_exists_case_sensitive(Path(file_path)):
        member_name = fs_path[last_sep_index + 1 :]
        return file_path, member_name

    init_file_path = fs_path[:last_sep_index] + "/__init__.py"
    if path_exists_case_sensitive(Path(init_file_path)):
        member_name = fs_path[last_sep_index + 1 :]
        return init_file_path, member_name

    raise ModguardParseError(
        f"Failed to translate module path {mod_path} into file path"
    )
