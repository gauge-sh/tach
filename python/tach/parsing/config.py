from __future__ import annotations

import re
from pathlib import Path
from typing import TYPE_CHECKING, Any

import tomli
import tomli_w

from tach import filesystem as fs
from tach.extension import (
    dump_project_config_to_toml as ext_dump_project_config_to_toml,
)
from tach.extension import (
    parse_project_config as ext_parse_project_config,
)

if TYPE_CHECKING:
    from tach.extension import ProjectConfig


def dump_project_config_to_toml(config: ProjectConfig) -> str:
    data = tomli.loads(ext_dump_project_config_to_toml(config))
    return tomli_w.dumps(data)


def migrate_deprecated_cache_backend(data: dict[str, Any]) -> dict[str, Any]:
    if "cache" in data:
        if "backend" in data["cache"]:
            data["cache"]["backend"] = "disk"
    return data


def migrate_deprecated_depends_on(data: dict[str, Any]) -> dict[str, Any]:
    if "modules" in data:
        for module in data["modules"]:
            if "depends_on" in module:
                for index, path in enumerate(module["depends_on"]):
                    if isinstance(path, str):
                        module["depends_on"][index] = {"path": path}
    return data


def migrate_deprecated_source_root(data: dict[str, Any]) -> dict[str, Any]:
    if "source_root" in data:
        if isinstance(data["source_root"], str):
            data["source_roots"] = [data["source_root"]]
            del data["source_root"]
    return data


def migrate_deprecated_yaml_config(filepath: Path) -> ProjectConfig:
    import yaml

    content = filepath.read_text()
    data = yaml.safe_load(content)

    try:
        data = migrate_deprecated_cache_backend(data)
        data = migrate_deprecated_depends_on(data)
        data = migrate_deprecated_source_root(data)
        toml_config = tomli_w.dumps(data)
        print("Auto-migrating deprecated YAML config to TOML...")
        filepath.with_suffix(".toml").write_text(toml_config)
        project_config, ext_migrated = ext_parse_project_config(
            filepath.with_suffix(".toml")
        )
        if ext_migrated:
            # This is a second migration pass, so we need to save the result
            filepath.with_suffix(".toml").write_text(
                dump_project_config_to_toml(project_config)
            )
    except TypeError as e:
        raise ValueError(f"Failed to parse deprecated YAML config: {e}")
    except ValueError as e:
        filepath.with_suffix(".toml").unlink()
        raise ValueError(f"Failed to parse deprecated YAML config: {e}")
    print("Deleting deprecated YAML config...")
    filepath.unlink()
    return project_config


def parse_project_config(root: Path | None = None) -> ProjectConfig | None:
    root = root or Path.cwd()
    file_path = fs.get_project_config_path(root)
    if file_path:
        # Standard TOML config found
        project_config, ext_migrated = ext_parse_project_config(file_path)
        if ext_migrated:
            # This is a second migration pass, so we need to save the result
            file_path.with_suffix(".toml").write_text(
                dump_project_config_to_toml(project_config)
            )
    else:
        # No TOML found, check for deprecated (YAML) config as a fallback
        file_path = fs.get_deprecated_project_config_path(root)
        if not file_path:
            return None
        # Return right away, this is a final ProjectConfig
        project_config = migrate_deprecated_yaml_config(file_path)
    return project_config


def extend_and_validate(
    exclude_paths: list[str] | None,
    project_excludes: list[str],
    use_regex_matching: bool,
) -> list[str]:
    if exclude_paths is not None:
        exclude_paths.extend(project_excludes)
    else:
        exclude_paths = project_excludes

    if not use_regex_matching:
        return exclude_paths

    for exclude_path in exclude_paths:
        try:
            re.compile(exclude_path)
        except re.error:
            raise ValueError(
                f"Invalid regex pattern: {exclude_path}. If you meant to use glob matching, set 'use_regex_matching' to false in your .toml file."
            )
    return exclude_paths
