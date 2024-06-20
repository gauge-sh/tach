from __future__ import annotations

from pathlib import Path

import yaml

from tach import filesystem as fs
from tach.core import ProjectConfig


def dump_project_config_to_yaml(config: ProjectConfig) -> str:
    # Using sort_keys=False here and depending on config.model_dump maintaining 'insertion order'
    # so that 'tag' appears before 'depends_on'
    # Instead, should provide custom yaml.Dumper & yaml.Representer or just write our own
    # Sort only constraints and dependencies alphabetically for now
    config.modules.sort(key=lambda mod: mod.path)
    for mod in config.modules:
        mod.depends_on.sort()
    # NOTE: setting 'exclude' explicitly here also interacts with the 'exclude_unset' option
    # being passed to 'model_dump'. It ensures that even on a fresh config, we will explicitly
    # show excluded paths.
    config.exclude = list(set(config.exclude)) if config.exclude else []
    config.exclude.sort()
    return yaml.dump(config.model_dump(exclude_unset=True), sort_keys=False)


def parse_project_config(root: Path | None = None) -> ProjectConfig | None:
    root = root or Path.cwd()
    file_path = fs.get_project_config_path(root)
    if not file_path:
        return None

    with open(file_path) as f:
        result = yaml.safe_load(f)
        if not result or not isinstance(result, dict):
            raise ValueError(f"Empty or invalid project config file: {file_path}")
    config = ProjectConfig(**result)  # type: ignore
    return config
