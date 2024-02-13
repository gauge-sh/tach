import os
import re
from dataclasses import dataclass
from typing import Optional

from . import filesystem as fs
from .core.boundary import BoundaryTrie, BoundaryNode
from .parsing.boundary import build_boundary_trie
from .parsing.imports import get_imports


@dataclass
class ErrorInfo:
    location: str = ""
    import_mod_path: str = ""
    boundary_path: str = ""
    exception_message: str = ""

    @property
    def message(self) -> str:
        if self.exception_message:
            return self.exception_message
        if not all((self.location, self.import_mod_path, self.boundary_path)):
            return f"Unexpected error: ({[self.location, self.import_mod_path, self.boundary_path]})"
        return f"Import '{self.import_mod_path}' in {self.location} is blocked by boundary '{self.boundary_path}'"


def check_import(
    boundary_trie: BoundaryTrie,
    import_mod_path: str,
    file_nearest_boundary: BoundaryNode,
    file_mod_path: str,
) -> Optional[BoundaryNode]:
    nearest_boundary = boundary_trie.find_nearest(import_mod_path)
    # An imported module is allowed only in the following cases:
    # * The module is not contained by a boundary [generally 3rd party]
    import_mod_has_boundary = nearest_boundary is not None

    # * The file's boundary is a child of the imported module's boundary
    import_mod_is_child_of_current = (
        import_mod_has_boundary
        and file_nearest_boundary.full_path.startswith(nearest_boundary.full_path)
    )

    # * The module is exported as public by its boundary and is allowed in the current path
    import_mod_public_member_definition = (
        next(
            (
                public_member
                for public_member_name, public_member in nearest_boundary.public_members.items()
                if re.match(rf"^{public_member_name}(\.[\w*]+)?$", import_mod_path)
            ),
            None,
        )
        if import_mod_has_boundary
        else None
    )
    import_mod_is_public_and_allowed = (
        import_mod_public_member_definition is not None
        and (
            import_mod_public_member_definition.allowlist is None
            or any(
                (
                    file_mod_path.startswith(allowed_path)
                    for allowed_path in import_mod_public_member_definition.allowlist
                )
            )
        )
    )

    if (
        not import_mod_has_boundary
        or import_mod_is_child_of_current
        or import_mod_is_public_and_allowed
    ):
        return None

    # In error case, return path of the violated boundary
    return nearest_boundary


def check(root: str, exclude_paths: Optional[list[str]] = None) -> list[ErrorInfo]:
    if not os.path.isdir(root):
        return [
            ErrorInfo(exception_message=f"The path {root} is not a valid directory.")
        ]

    # This 'canonicalizes' the path arguments, resolving directory traversal
    root = fs.canonical(root)
    exclude_paths = list(map(fs.canonical, exclude_paths)) if exclude_paths else None

    pyfiles = list(fs.walk_pyfiles(root, exclude_paths=exclude_paths))
    boundary_trie = build_boundary_trie(root, pyfiles=pyfiles)

    errors: list[ErrorInfo] = []
    for file_path in pyfiles:
        mod_path = fs.file_to_module_path(file_path)
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
                # This import is OK
                continue

            errors.append(
                ErrorInfo(
                    import_mod_path=import_mod_path,
                    boundary_path=violated_boundary.full_path,
                    location=file_path,
                )
            )

    return errors
