import os
from typing import Generator
from modguard import public


public()


def canonical(file_path: str) -> str:
    return os.path.relpath(os.path.realpath(file_path))


def walk_pyfiles(
    root: str, exclude_paths: list[str] = None
) -> Generator[tuple[str, str], None, None]:
    for dirpath, _, filenames in os.walk(root):
        for filename in filenames:
            file_path = canonical(os.path.join(dirpath, filename))
            if exclude_paths is not None and any(
                file_path.startswith(exclude_path) for exclude_path in exclude_paths
            ):
                # Treat excluded paths as invisible
                continue
            if filename.endswith(".py"):
                yield dirpath, filename


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
