import os
from typing import Iterable, Optional

import yaml

from modguard import filesystem as fs
from modguard.check import check
from modguard.constants import CONFIG_FILE_NAME
from modguard.core import ScopeDependencyRules
from modguard.errors import ModguardError
from modguard.parsing import parse_project_config


def update_project_config(root: str, tags: set[str]):
    current_dir = os.getcwd()
    try:
        fs.chdir(root)
        project_config = parse_project_config()
        check_errors = check(
            root,
            project_config=project_config,
            exclude_paths=project_config.exclude,
        )
        for error in check_errors:
            if error.is_tag_error:
                invalid_tags = set(error.invalid_tags)
                existing_dependencies = set(
                    project_config.constraints.get(
                        error.source_tag, ScopeDependencyRules(depends_on=[])
                    ).depends_on
                )
                if error.source_tag in tags:
                    # This is updating the config for a new tag
                    project_config.constraints[error.source_tag] = ScopeDependencyRules(
                        depends_on=list(existing_dependencies | invalid_tags)
                    )
                if invalid_tags & tags:
                    # This is updating the config for an existing tag
                    project_config.constraints[error.source_tag] = ScopeDependencyRules(
                        depends_on=list(existing_dependencies | (invalid_tags & tags))
                    )

        modguard_yml_path = os.path.join(root, f"{CONFIG_FILE_NAME}.yml")
        modguard_yml_content = yaml.dump(project_config.model_dump())
        fs.write_file(modguard_yml_path, modguard_yml_content)

        check_errors = check(
            root, project_config=project_config, exclude_paths=project_config.exclude
        )
        if check_errors:
            return (
                "Could not auto-detect all dependencies, "
                "use 'modguard check' to finish initialization manually."
            )
    except Exception as e:
        fs.chdir(current_dir)
        raise e
    fs.chdir(current_dir)


def add_packages(paths: set[str], tags: Optional[set[str]]) -> Iterable[str]:
    new_tags: set[str] = set()
    # Validate paths
    for path in paths:
        fs.validate_path_for_add(path=path)
    # Build packages
    for path in paths:
        new_tag = fs.build_package(path=path, tags=tags)
        if new_tag:
            new_tags.add(new_tag)
    # Update project config
    project_root = fs.find_project_config_root(path=".")
    if not project_root:
        raise ModguardError(f"{CONFIG_FILE_NAME} not found.")
    warning = update_project_config(root=project_root, tags=tags if tags else new_tags)
    if warning:
        return [warning]
    return []
