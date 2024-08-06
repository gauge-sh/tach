from __future__ import annotations

from pathlib import Path
from typing import Any

import yaml

from tach import filesystem as fs
from tach.constants import ROOT_MODULE_SENTINEL_TAG, TACH_YML_SCHEMA_URL
from tach.core import (
    CacheConfig,
    Dependency,
    ExternalDependencyConfig,
    ModuleConfig,
    ProjectConfig,
)
from tach.extension import parse_project_config as ext_parse_project_config


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
        mod.depends_on.sort(key=lambda dep: dep.path)
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


# TODO remove after next major version upgrade
def migrate_config(result: dict[Any, Any]) -> dict[Any, Any]:
    if "modules" in result:
        for module in result["modules"]:
            if "depends_on" in module:
                for index, path in enumerate(module["depends_on"]):
                    if isinstance(path, str):
                        module["depends_on"][index] = {"path": path}
    if "source_root" in result and isinstance(result["source_root"], str):
        result["source_roots"] = [result["source_root"]]
        del result["source_root"]
    return result


def parse_project_config(root: Path | None = None) -> ProjectConfig | None:
    root = root or Path.cwd()
    file_path = fs.get_project_config_path(root)
    if not file_path:
        return None

    ext_project_config = ext_parse_project_config(str(file_path))

    return ProjectConfig(
        modules=[
            ModuleConfig(
                path=module.path,
                depends_on=[
                    Dependency(path=dep.path, deprecated=dep.deprecated)
                    for dep in module.depends_on
                ],
                strict=module.strict,
            )
            for module in ext_project_config.modules
        ],
        cache=CacheConfig(
            file_dependencies=ext_project_config.cache.file_dependencies,
            env_dependencies=ext_project_config.cache.env_dependencies,
        ),
        external=ExternalDependencyConfig(
            exclude=ext_project_config.external.exclude,
        ),
        exclude=ext_project_config.exclude,
        source_roots=[Path(root) for root in ext_project_config.source_roots],
        exact=ext_project_config.exact,
        disable_logging=ext_project_config.disable_logging,
        ignore_type_checking_imports=ext_project_config.ignore_type_checking_imports,
        forbid_circular_dependencies=ext_project_config.forbid_circular_dependencies,
    )
