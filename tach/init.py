import os
from dataclasses import field, dataclass
from typing import Optional

import yaml

from tach import errors
from tach import filesystem as fs
from tach.check import check
from tach.constants import PACKAGE_FILE_NAME, CONFIG_FILE_NAME
from tach.core import ProjectConfig, ScopeDependencyRules

__package_yml_template = """tags: ['{dir_name}']\n"""


@dataclass
class PackageInitResult:
    package_paths: list[str] = field(default_factory=list)
    warnings: list[str] = field(default_factory=list)


def init_packages(
    root: str, depth: int, exclude_paths: Optional[list[str]] = None
) -> PackageInitResult:
    package_paths: list[str] = []
    warnings: list[str] = []
    for dir_path in fs.walk_pypackages(root, depth=depth, exclude_paths=exclude_paths):
        package_yml_path = os.path.join(dir_path, f"{PACKAGE_FILE_NAME}.yml")
        package_paths.append(dir_path)
        if os.path.exists(package_yml_path):
            warnings.append(f"Package file '{package_yml_path}' already exists.")
            continue
        package_yml_content = __package_yml_template.format(
            dir_name=dir_path.replace(os.path.sep, ".")
        )
        fs.write_file(package_yml_path, package_yml_content)

    return PackageInitResult(package_paths=package_paths, warnings=warnings)


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
                project_config.constraints.get(
                    error.source_tag, ScopeDependencyRules(depends_on=[])
                ).depends_on
            )
            project_config.constraints[error.source_tag] = ScopeDependencyRules(
                depends_on=list(existing_dependencies | set(error.invalid_tags))
            )

    tach_yml_path = os.path.join(root, f"{CONFIG_FILE_NAME}.yml")
    tach_yml_content = yaml.dump(project_config.model_dump())
    fs.write_file(tach_yml_path, tach_yml_content)

    check_errors = check(
        root, project_config=project_config, exclude_paths=exclude_paths
    )
    if check_errors:
        return InitRootResult(
            warnings=[
                "Could not auto-detect all dependencies, use 'tach check' to finish initialization manually."
            ]
        )

    return InitRootResult(warnings=[])


def init_project(
    root: str, depth: Optional[int] = None, exclude_paths: Optional[list[str]] = None
) -> list[str]:
    if not os.path.isdir(root):
        raise errors.TachSetupError(f"The path {root} is not a directory.")

    warnings: list[str] = []

    if depth is None:
        package_init_result = init_packages(root, depth=1, exclude_paths=exclude_paths)
        warnings.extend(package_init_result.warnings)
        if len(package_init_result.package_paths) == 1:
            result = init_packages(
                package_init_result.package_paths[0],
                depth=1,
                exclude_paths=exclude_paths,
            )
            warnings.extend(result.warnings)
    else:
        package_init_result = init_packages(
            root, depth=depth, exclude_paths=exclude_paths
        )
        warnings.extend(package_init_result.warnings)

    init_root_result = init_root(root, exclude_paths=exclude_paths)
    warnings.extend(init_root_result.warnings)

    return warnings
