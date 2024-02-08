import os
import errors
from .check import check_import
from .core import PublicMember
from .parsing import utils
from .parsing.boundary import ensure_boundary, build_boundary_trie
from .parsing.imports import get_imports
from .parsing.public import mark_as_public


def init_project(root: str, exclude_paths: list[str] = None):
    # Core functionality:
    # * do nothing in any package already having a Boundary
    # * import and call Boundary in __init__.py for all other packages
    # * import and decorate public on all externally imported functions and classes
    if not os.path.isdir(root):
        return errors.ModguardSetupError(f"The path {root} is not a directory.")

    # This 'canonicalizes' the path arguments, resolving directory traversal
    root = utils.canonical(root)
    exclude_paths = list(map(utils.canonical, exclude_paths)) if exclude_paths else None

    boundary_trie = build_boundary_trie(root, exclude_paths=exclude_paths)
    initial_boundary_paths = [boundary.full_path for boundary in boundary_trie]

    for dirpath in utils.walk_pypackages(root, exclude_paths=exclude_paths):
        added_boundary = ensure_boundary(dirpath + "/__init__.py")
        if added_boundary:
            dir_mod_path = utils.file_to_module_path(dirpath)
            boundary_trie.insert(dir_mod_path)

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
            mark_as_public(file_path, member_name)
            violated_boundary.add_public_member(PublicMember(name=import_mod_path))
