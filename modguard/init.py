from collections import defaultdict
from dataclasses import dataclass
from enum import Enum
import os
from itertools import chain
from typing import Optional

from modguard import errors, filesystem as fs
from modguard.check import check_import
from modguard.core import PublicMember
from modguard.parsing.boundary import add_boundary, build_boundary_trie
from modguard.parsing.imports import get_imports
from modguard.parsing.public import mark_as_public


class WriteOperation(Enum):
    BOUNDARY = "boundary"
    PUBLIC = "public"


@dataclass(eq=True, frozen=True)
class FileWriteInformation:
    location: str
    operation: WriteOperation
    member_name: str = ""


def deduplicate_writes(
    writes: list[FileWriteInformation],
) -> list[FileWriteInformation]:
    public_writes: defaultdict[str, list[FileWriteInformation]] = defaultdict(list)
    result: list[FileWriteInformation] = []
    # Basic uniqueness simplifies later checks
    writes = list(set(writes))
    for write in writes:
        if write.operation == WriteOperation.BOUNDARY:
            # Uniqueness check means all boundary writes should be kept
            result.append(write)
        elif write.operation == WriteOperation.PUBLIC:
            root_public = FileWriteInformation(
                location=write.location,
                operation=WriteOperation.PUBLIC,
                member_name="",
            )
            if write.location in public_writes and public_writes[write.location] == [
                root_public
            ]:
                # Root already public, skip this write
                continue

            if write.member_name in ["", "*"]:
                # A blank public write clears all other public writes for the location
                public_writes[write.location] = [root_public]
            else:
                public_writes[write.location].append(write)
    return [*result, *chain(*public_writes.values())]


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
    initial_boundary_paths = [
        boundary.full_path for boundary in boundary_trie if boundary.full_path
    ]

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

    for file_path in fs.walk_pyfiles(root, exclude_paths=exclude_paths):
        mod_path = fs.file_to_module_path(file_path)
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

            member_name = ""
            try:
                file_path, member_name = fs.module_to_file_path(
                    import_mod_path, find_package_init=True
                )
                write_operations.append(
                    FileWriteInformation(
                        location=file_path,
                        operation=WriteOperation.PUBLIC,
                        member_name=member_name,
                    )
                )
                violated_boundary.add_public_member(PublicMember(name=import_mod_path))
            except errors.ModguardError:
                warnings.append(
                    f"Warning: Skipped member {member_name or import_mod_path} in {file_path}; could not mark as public"
                )
    # After we've completed our pass on inserting boundaries and public members, write to files
    for write_op in deduplicate_writes(write_operations):
        try:
            if write_op.operation == WriteOperation.BOUNDARY:
                add_boundary(write_op.location)
            elif write_op.operation == WriteOperation.PUBLIC:
                mark_as_public(write_op.location, write_op.member_name)
        except errors.ModguardError:
            warnings.append(
                f"Warning: Could not mark {write_op.operation.value}"
                f"{'({member})'.format(member=write_op.member_name) if write_op.member_name else ''}"
                f" in {write_op.location}"
            )

    return warnings
