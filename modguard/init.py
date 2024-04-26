import os
from typing import Optional

from modguard import errors


def init_project(root: str, exclude_paths: Optional[list[str]] = None) -> list[str]:
    # Core functionality:
    # * do nothing in any package already having a Boundary
    # * import and call Boundary in __init__.py for all other packages
    # * import and decorate public on all externally imported members
    if not os.path.isdir(root):
        raise errors.ModguardSetupError(f"The path {root} is not a directory.")

    return ["Not Implemented"]
