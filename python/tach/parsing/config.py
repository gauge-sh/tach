from __future__ import annotations

from pathlib import Path
from typing import TYPE_CHECKING

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
        toml_config = tomli_w.dumps(data)
        print("Auto-migrating deprecated YAML config to TOML...")
        filepath.with_suffix(".toml").write_text(toml_config)
        project_config = ext_parse_project_config(filepath.with_suffix(".toml"))
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
        project_config = ext_parse_project_config(file_path)
    else:
        # No TOML found, check for deprecated (YAML) config as a fallback
        file_path = fs.get_deprecated_project_config_path(root)
        if not file_path:
            return None
        # Return right away, this is a final ProjectConfig
        project_config = migrate_deprecated_config(file_path)
    return project_config
