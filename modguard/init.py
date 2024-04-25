from dataclasses import dataclass
from enum import Enum
import os
from typing import Optional

from modguard import errors, filesystem as fs
from modguard.parsing.boundary import add_boundary, build_boundary_trie


class WriteOperation(Enum):
    BOUNDARY = "boundary"


@dataclass(eq=True, frozen=True)
class FileWriteInformation:
    location: str
    operation: WriteOperation
    member_name: str = ""


def init_project(root: str, exclude_paths: Optional[list[str]] = None) -> list[str]:
    # Core functionality:
    # * do nothing in any package already having a Boundary
    # * import and call Boundary in __init__.py for all other packages
    # * import and decorate public on all externally imported members
    if not os.path.isdir(root):
        raise errors.ModguardSetupError(f"The path {root} is not a directory.")

    warnings: list[str] = []

    # This 'canonicalizes' the path arguments, resolving directory traversal
    root = fs.canonical(root)
    exclude_paths = list(map(fs.canonical, exclude_paths)) if exclude_paths else None

    write_operations: list[FileWriteInformation] = []

    boundary_trie = build_boundary_trie(root, exclude_paths=exclude_paths)

    for dirpath in fs.walk_pypackages(root, exclude_paths=exclude_paths):
        filepath = dirpath + "/__init__.py"
        dir_mod_path = fs.file_to_module_path(dirpath)
        if not boundary_trie.get(dir_mod_path):
            boundary_trie.insert(dir_mod_path)
            write_operations.append(
                FileWriteInformation(
                    location=filepath, operation=WriteOperation.BOUNDARY
                )
            )

    # After we've completed our pass on inserting boundaries, write to files
    for write_op in write_operations:
        try:
            if write_op.operation == WriteOperation.BOUNDARY:
                add_boundary(write_op.location)
        except errors.ModguardError:
            warnings.append(
                f"Warning: Could not mark {write_op.operation.value}"
                f"{'({member})'.format(member=write_op.member_name) if write_op.member_name else ''}"
                f" in {write_op.location}"
            )

    return warnings
