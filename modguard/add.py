import os
from typing import Iterable, Optional

import yaml

from modguard import filesystem as fs
from modguard.check import check
from modguard.constants import CONFIG_FILE_NAME
from modguard.core import ScopeDependencyRules
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
                if error.source_tag in tags or invalid_tags & tags:
                    existing_dependencies = set(
                        project_config.constraints.get(
                            error.source_tag, ScopeDependencyRules(depends_on=[])
                        ).depends_on
                    )
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
            return "Could not auto-detect all dependencies, use 'modguard check' to finish initialization manually."
    except Exception as e:
        fs.chdir(current_dir)
        raise e
    fs.chdir(current_dir)



def add_modules(paths: set[str], tags: Optional[set[str]]) -> Iterable[str]:
    # Validate paths
    for path in paths:
        fs.validate_path_for_add(path=path)
    # Build modules
    for path in paths:
        fs.build_module(path=path, tags=tags)
    # Update project config
    project_root = fs.find_project_root()
    warning = update_project_config(root=project_root, tags=tags)
    if warning:
        return [warning]
    return []
