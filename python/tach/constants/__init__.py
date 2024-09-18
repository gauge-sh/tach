from __future__ import annotations

PACKAGE_NAME = "tach"
TOOL_NAME = "tach"
CONFIG_FILE_NAME = TOOL_NAME
PACKAGE_FILE_NAME = "package"
ROOT_MODULE_SENTINEL_TAG = "<root>"
TACH_YML_SCHEMA_URL = "https://raw.githubusercontent.com/gauge-sh/tach/v0.11.0/public/tach-yml-schema.json"

DEFAULT_EXCLUDE_PATHS = ["tests", "docs", ".*__pycache__", ".*egg-info"]

__all__ = [
    "PACKAGE_NAME",
    "TOOL_NAME",
    "CONFIG_FILE_NAME",
    "PACKAGE_FILE_NAME",
    "ROOT_MODULE_SENTINEL_TAG",
    "TACH_YML_SCHEMA_URL",
    "DEFAULT_EXCLUDE_PATHS",
]
