import os
from dataclasses import field, dataclass
from typing import Optional

from modguard import errors
from modguard import filesystem as fs
from modguard.constants import MODULE_FILE_NAME


__module_yml_template = """tags: ['{dir_name}']\n"""


@dataclass
class ModuleInitResult:
    module_paths: list[str] = field(default_factory=list)
    warnings: list[str] = field(default_factory=list)


def init_modules(
    root: str, depth: int, exclude_paths: Optional[list[str]] = None
) -> ModuleInitResult:
    module_paths = []
    warnings = []
    for dir_path in fs.walk_pypackages(root, depth=depth, exclude_paths=exclude_paths):
        module_yml_path = os.path.join(dir_path, f"{MODULE_FILE_NAME}.yml")
        module_paths.append(dir_path)
        if os.path.exists(module_yml_path):
            warnings.append(f"Module file '{module_yml_path}' already exists.")
            continue
        module_yml_content = __module_yml_template.format(
            dir_name=dir_path.replace(os.path.sep, ".")
        )
        fs.write_file(module_yml_path, module_yml_content)

    return ModuleInitResult(module_paths=module_paths, warnings=warnings)


def init_project(
    root: str, depth: Optional[int] = None, exclude_paths: Optional[list[str]] = None
) -> list[str]:
    if not os.path.isdir(root):
        raise errors.ModguardSetupError(f"The path {root} is not a directory.")

    warnings: list[str] = []

    if depth is None:
        result = init_modules(root, depth=1, exclude_paths=exclude_paths)
        warnings.extend(result.warnings)
        print(result.module_paths)
        if len(result.module_paths) == 1:
            result = init_modules(
                result.module_paths[0], depth=1, exclude_paths=exclude_paths
            )
            warnings.extend(result.warnings)
        return warnings

    result = init_modules(root, depth=depth, exclude_paths=exclude_paths)
    return result.warnings
