from __future__ import annotations

from pathlib import Path

import tomli_w

from tach import filesystem as fs
from tach.constants import ROOT_MODULE_SENTINEL_TAG
from tach.core import (
    CacheConfig,
    Dependency,
    ExternalDependencyConfig,
    ModuleConfig,
    ProjectConfig,
)
from tach.extension import parse_project_config as ext_parse_project_config


def dump_project_config_to_toml(config: ProjectConfig) -> str:
    config.modules.sort(
        key=lambda mod: (mod.path == ROOT_MODULE_SENTINEL_TAG, mod.path)
    )
    for mod in config.modules:
        mod.depends_on.sort(key=lambda dep: dep.path)

    config.exclude.sort()
    config.source_roots.sort()

    return tomli_w.dumps(
        config.model_dump(exclude_unset=True),
    )


def migrate_deprecated_config(filepath: Path) -> ProjectConfig:
    import yaml

    content = filepath.read_text()
    data = yaml.safe_load(content)

    try:
        if "cache" in data:
            if "backend" in data["cache"]:
                # Force cache backend to 'disk' (original value was 'local')
                data["cache"]["backend"] = "disk"
        # Old migrations
        if "modules" in data:
            for module in data["modules"]:
                if "depends_on" in module:
                    for index, path in enumerate(module["depends_on"]):
                        if isinstance(path, str):
                            module["depends_on"][index] = {"path": path}
        if "source_root" in data and isinstance(data["source_root"], str):
            data["source_roots"] = [data["source_root"]]
            del data["source_root"]
        project_config = ProjectConfig(**data)  # type: ignore
    except TypeError as e:
        raise ValueError(f"Failed to parse deprecated YAML config: {e}")

    print("Auto-migrating deprecated YAML config to TOML...")
    filepath.with_suffix(".toml").write_text(
        dump_project_config_to_toml(project_config)
    )
    print("Deleting deprecated YAML config...")
    filepath.unlink()
    return project_config


def parse_project_config(root: Path | None = None) -> ProjectConfig | None:
    root = root or Path.cwd()
    file_path = fs.get_project_config_path(root)

    if file_path:
        # Standard TOML config found
        project_config = ext_parse_project_config(str(file_path))
    else:
        # No TOML found, check for deprecated (YAML) config as a fallback
        file_path = fs.get_deprecated_project_config_path(root)
        if not file_path:
            return None
        # Return right away, this is a final ProjectConfig
        return migrate_deprecated_config(file_path)

    # 'with_derived_unset_fields' is used here explicitly
    #   to ensure that later dumps will not include "unset" fields
    #   (i.e. fields that are default values and not declared ALWAYS_DUMP)
    return ProjectConfig.with_derived_unset_fields(
        {
            "modules": [
                ModuleConfig.with_derived_unset_fields(
                    {
                        "path": module.path,
                        "depends_on": [
                            Dependency.with_derived_unset_fields(
                                {"path": dep.path, "deprecated": dep.deprecated}
                            )
                            for dep in module.depends_on
                        ],
                        "strict": module.strict,
                    }
                )
                for module in project_config.modules
            ],
            "cache": CacheConfig.with_derived_unset_fields(
                {
                    "file_dependencies": project_config.cache.file_dependencies,
                    "env_dependencies": project_config.cache.env_dependencies,
                }
            ),
            "external": ExternalDependencyConfig.with_derived_unset_fields(
                {
                    "exclude": project_config.external.exclude,
                }
            ),
            "exclude": project_config.exclude,
            "source_roots": [Path(root) for root in project_config.source_roots],
            "exact": project_config.exact,
            "disable_logging": project_config.disable_logging,
            "ignore_type_checking_imports": project_config.ignore_type_checking_imports,
            "forbid_circular_dependencies": project_config.forbid_circular_dependencies,
            "use_regex_matching": project_config.use_regex_matching,
        }
    )
