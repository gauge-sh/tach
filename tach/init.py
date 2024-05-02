import os
from dataclasses import field, dataclass
from typing import Optional


from tach import errors
from tach import filesystem as fs
from tach.check import check
from tach.colors import BCOLORS
from tach.constants import PACKAGE_FILE_NAME, CONFIG_FILE_NAME
from tach.parsing import dump_project_config_to_yaml, parse_config

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
            warnings.append(
                f"{BCOLORS.OKCYAN}Package file '{package_yml_path}' already exists.{BCOLORS.ENDC}"
            )
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
    project_config_path = fs.get_project_config_yml_path(root)
    if project_config_path:
        return InitRootResult(
            warnings=[
                f"{BCOLORS.OKCYAN}Project already contains {CONFIG_FILE_NAME}.yml{BCOLORS.ENDC}"
            ]
        )

    config = parse_config(root=root, exclude_paths=exclude_paths)
    check_errors = check(root, config=config)
    for error in check_errors:
        if error.is_tag_error:
            config.project.add_dependencies_to_tag(error.source_tag, error.invalid_tags)

    # TODO: ClI option for initializing TOML or YML
    tach_yml_path = os.path.join(root, f"{CONFIG_FILE_NAME}.yml")
    tach_yml_content = dump_project_config_to_yaml(config.project)
    fs.write_file(tach_yml_path, tach_yml_content)

    # Relies on mutation
    check_errors = check(root, config=config)
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

    if exclude_paths is None:
        exclude_paths = ["tests/", "docs/"]

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
