from __future__ import annotations

from pathlib import Path

import yaml

from tach import filesystem as fs
from tach.constants import ROOT_MODULE_SENTINEL_TAG, TACH_YML_SCHEMA_URL
from tach.core import ProjectConfig


class TachYamlDumper(yaml.Dumper):
    def increase_indent(self, flow: bool = False, indentless: bool = False):
        return super().increase_indent(flow, False)


def dump_project_config_to_yaml(config: ProjectConfig) -> str:
    # Using sort_keys=False here and depending on config.model_dump maintaining 'insertion order'
    # so that 'tag' appears before 'depends_on'
    # Instead, should provide custom yaml.Dumper & yaml.Representer or just write our own
    # Sort only constraints and dependencies alphabetically for now
    config.modules.sort(
        key=lambda mod: (mod.path == ROOT_MODULE_SENTINEL_TAG, mod.path)
    )
    for mod in config.modules:
        mod.depends_on.sort()
    # NOTE: setting 'exclude' explicitly here also interacts with the 'exclude_unset' option
    # being passed to 'model_dump'. It ensures that even on a fresh config, we will explicitly
    # show excluded paths.
    config.exclude = list(set(config.exclude)) if config.exclude else []
    config.exclude.sort()
    language_server_directive = (
        f"# yaml-language-server: $schema={TACH_YML_SCHEMA_URL}\n"
    )
    yaml_content = yaml.dump(
        config.model_dump(exclude_unset=True),
        Dumper=TachYamlDumper,
        sort_keys=False,
        default_flow_style=False,
        indent=2,
    )
    return language_server_directive + yaml_content


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
