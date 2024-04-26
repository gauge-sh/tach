import os
from dataclasses import field, dataclass
from typing import Optional

import yaml

from modguard import errors
from modguard import filesystem as fs
from modguard.check import check
from modguard.constants import MODULE_FILE_NAME, CONFIG_FILE_NAME
from modguard.core import ProjectConfig, ScopeDependencyRules

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


@dataclass
class InitRootResult:
    warnings: list[str] = field(default_factory=list)


def init_root(root: str, exclude_paths: Optional[list[str]] = None) -> InitRootResult:
    project_config_path = fs.get_project_config_path(root)
    if project_config_path:
        return InitRootResult(
            warnings=[f"Project already contains {CONFIG_FILE_NAME}.yml"]
        )

    project_config = ProjectConfig()
    check_errors = check(
        root, project_config=project_config, exclude_paths=exclude_paths
    )
    for error in check_errors:
        if error.is_tag_error:
            existing_dependencies = set(
                project_config.constraints.get(error.source_tag, {})
            )
            project_config.constraints[error.source_tag] = ScopeDependencyRules(
                depends_on=list(existing_dependencies | set(error.invalid_tags))
            )

    modguard_yml_path = os.path.join(root, CONFIG_FILE_NAME)
    modguard_yml_content = yaml.dump(project_config.model_dump())
    fs.write_file(modguard_yml_path, modguard_yml_content)

    check_errors = check(
        root, project_config=project_config, exclude_paths=exclude_paths
    )
    if check_errors:
        return InitRootResult(
            warnings=[
                "Could not auto-detect all dependencies, use 'modguard check' to finish initialization manually."
            ]
        )

    return InitRootResult(warnings=[])


def init_project(
    root: str, depth: Optional[int] = None, exclude_paths: Optional[list[str]] = None
) -> list[str]:
    if not os.path.isdir(root):
        raise errors.ModguardSetupError(f"The path {root} is not a directory.")

    warnings: list[str] = []

    if depth is None:
        module_init_result = init_modules(root, depth=1, exclude_paths=exclude_paths)
        warnings.extend(module_init_result.warnings)
        if len(module_init_result.module_paths) == 1:
            result = init_modules(
                module_init_result.module_paths[0], depth=1, exclude_paths=exclude_paths
            )
            warnings.extend(result.warnings)
    else:
        module_init_result = init_modules(
            root, depth=depth, exclude_paths=exclude_paths
        )
        warnings.extend(module_init_result.warnings)

    init_root_result = init_root(root, exclude_paths=exclude_paths)
    warnings.extend(init_root_result.warnings)

    return warnings
