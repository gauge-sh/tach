import os
from dataclasses import dataclass

from .parsing.boundary import build_boundary_trie
from .parsing.imports import get_imports
from .parsing import utils


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


def check(root: str, exclude_paths: list[str] = None) -> list[ErrorInfo]:
    if not os.path.isdir(root):
        return [ErrorInfo(exception_message=f"The path {root} is not a directory.")]

    # This 'canonicalizes' the path arguments, resolving directory traversal
    root = utils.canonical(root)
    exclude_paths = list(map(utils.canonical, exclude_paths)) if exclude_paths else None

    boundary_trie = build_boundary_trie(root, exclude_paths=exclude_paths)

    errors = []
    for dirpath, filename in utils.walk_pyfiles(root, exclude_paths=exclude_paths):
        file_path = os.path.join(dirpath, filename)
        current_mod_path = utils.file_to_module_path(file_path)
        current_nearest_boundary = boundary_trie.find_nearest(current_mod_path)
        assert (
            current_nearest_boundary is not None
        ), f"Checking file ({file_path}) outside of boundaries!"
        import_mod_paths = get_imports(file_path)
        for mod_path in import_mod_paths:
            nearest_boundary = boundary_trie.find_nearest(mod_path)
            # An imported module is allowed only in the following cases:
            # * The module is not contained by a boundary [generally 3rd party]
            import_mod_has_boundary = nearest_boundary is not None

            # * The module's boundary is a child of the current boundary
            import_mod_is_child_of_current = (
                import_mod_has_boundary
                and current_nearest_boundary.full_path.startswith(
                    nearest_boundary.full_path
                )
            )

            # * The module is exported as public by its boundary and is allowed in the current path
            import_mod_public_member_definition = (
                next(
                    (
                        public_member
                        for public_member_name, public_member in nearest_boundary.public_members.items()
                        if mod_path.startswith(public_member_name)
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
                            current_mod_path.startswith(allowed_path)
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
                # This import is OK
                continue

            errors.append(
                ErrorInfo(
                    import_mod_path=mod_path,
                    boundary_path=nearest_boundary.full_path,
                    location=file_path,
                )
            )

    return errors
