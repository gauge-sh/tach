import os
from typing import Generator
from modguard import public, errors


public()


def canonical(file_path: str) -> str:
    return os.path.relpath(os.path.realpath(file_path))


def walk_pyfiles(
    root: str, exclude_paths: list[str] = None
) -> Generator[str, None, None]:
    for dirpath, _, filenames in os.walk(root):
        for filename in filenames:
            file_path = canonical(os.path.join(dirpath, filename))
            if exclude_paths is not None and any(
                file_path.startswith(exclude_path) for exclude_path in exclude_paths
            ):
                # Treat excluded paths as invisible
                continue
            if filename.endswith(".py"):
                yield file_path


def walk_pypackages(
    root: str, exclude_paths: list[str] = None
) -> Generator[str, None, None]:
    for filepath in walk_pyfiles(root, exclude_paths=exclude_paths):
        init_file_ending = f"{os.path.sep}__init__.py"
        if filepath.endswith(init_file_ending):
            yield filepath[: -len(init_file_ending)]


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


def module_to_file_path(
    mod_path: str, find_package_init: bool = False
) -> tuple[str, str]:
    # Assumes that the mod_path is correctly formatted and refers to an actual module
    fs_path = mod_path.replace(".", os.path.sep)

    # mod_path may refer to a package
    if os.path.isdir(fs_path):
        return (
            os.path.join(fs_path(), "__init__.py") if find_package_init else fs_path
        ), ""

    # mod_path may refer to a file module
    file_path = fs_path + ".py"
    if os.path.exists(file_path):
        return file_path, ""

    # mod_path may refer to a member within a file module
    last_sep_index = fs_path.rfind(os.path.sep)
    file_path = fs_path[:last_sep_index] + ".py"
    if os.path.exists(file_path):
        member_name = fs_path[last_sep_index + 1 :]
        return file_path, member_name

    init_file_path = fs_path[:last_sep_index] + "/__init__.py"
    if os.path.exists(init_file_path):
        member_name = fs_path[last_sep_index + 1 :]
        return init_file_path, member_name

    raise errors.ModguardParseError(
        f"Failed to translate module path {mod_path} into file path"
    )
