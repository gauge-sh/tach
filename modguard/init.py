from dataclasses import dataclass
from enum import Enum
import os
from typing import Optional

from . import errors
from .check import check_import
from .core import PublicMember
from .parsing import utils
from .parsing.boundary import add_boundary, has_boundary, build_boundary_trie
from .parsing.imports import get_imports
from .parsing.public import mark_as_public


class WriteOperation(Enum):
    BOUNDARY = "boundary"
    PUBLIC = "public"


@dataclass
class FileWriteInformation:
    location: str
    operation: WriteOperation
    member_name: str = ""


def init_project(root: str, exclude_paths: Optional[list[str]] = None):
    # Core functionality:
    # * do nothing in any package already having a Boundary
    # * import and call Boundary in __init__.py for all other packages
    # * import and decorate public on all externally imported functions and classes
    if not os.path.isdir(root):
        raise errors.ModguardSetupError(f"The path {root} is not a directory.")

    # This 'canonicalizes' the path arguments, resolving directory traversal
    root = utils.canonical(root)
    exclude_paths = list(map(utils.canonical, exclude_paths)) if exclude_paths else None

    write_operations: list[FileWriteInformation] = []

    boundary_trie = build_boundary_trie(root, exclude_paths=exclude_paths)
    initial_boundary_paths = [
        boundary.full_path for boundary in boundary_trie if boundary.full_path
    ]

    for dirpath in utils.walk_pypackages(root, exclude_paths=exclude_paths):
        filepath = dirpath + "/__init__.py"
        if not has_boundary(filepath):
            dir_mod_path = utils.file_to_module_path(dirpath)
            boundary_trie.insert(dir_mod_path)
            write_operations.append(
                FileWriteInformation(
                    location=filepath, operation=WriteOperation.BOUNDARY
                )
            )

    for file_path in utils.walk_pyfiles(root, exclude_paths=exclude_paths):
        mod_path = utils.file_to_module_path(file_path)
        # If this file belongs to a Boundary which existed
        # before calling init_project, ignore the file and move on
        if any(
            (
                mod_path.startswith(initial_boundary_path)
                for initial_boundary_path in initial_boundary_paths
            )
        ):
            continue

        nearest_boundary = boundary_trie.find_nearest(mod_path)
        assert (
            nearest_boundary is not None
        ), f"Checking file ({file_path}) outside of boundaries!"
        import_mod_paths = get_imports(file_path)
        for import_mod_path in import_mod_paths:
            violated_boundary = check_import(
                boundary_trie=boundary_trie,
                import_mod_path=import_mod_path,
                file_nearest_boundary=nearest_boundary,
                file_mod_path=mod_path,
            )
            if violated_boundary is None:
                # This import is fine, no need to mark anything as public
                continue

            file_path, member_name = utils.module_to_file_path(import_mod_path)
            try:
                write_operations.append(
                    FileWriteInformation(
                        location=file_path,
                        operation=WriteOperation.PUBLIC,
                        member_name=member_name,
                    )
                )
                violated_boundary.add_public_member(PublicMember(name=import_mod_path))
            except errors.ModguardError:
                print(
                    f"Skipping member {member_name} in {file_path}; could not mark as public"
                )
    # After we've completed our pass on inserting boundaries and public members, write to files
    for write_op in write_operations:
        try:
            if write_op.operation == WriteOperation.BOUNDARY:
                add_boundary(write_op.location)
            if write_op.operation == WriteOperation.PUBLIC:
                mark_as_public(write_op.location, write_op.member_name)
        except errors.ModguardError:
            print(f"Error marking {write_op.operation} in {write_op.location}")
