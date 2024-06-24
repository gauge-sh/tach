from __future__ import annotations

PACKAGE_NAME = "tach"
TOOL_NAME = "tach"
CONFIG_FILE_NAME = TOOL_NAME
PACKAGE_FILE_NAME = "package"
ROOT_MODULE_SENTINEL_TAG = "<root>"

DEFAULT_EXCLUDE_PATHS = ["tests", "docs", "venv", ".*__pycache__", ".*egg-info"]

__all__ = [
    "PACKAGE_NAME",
    "TOOL_NAME",
    "CONFIG_FILE_NAME",
    "PACKAGE_FILE_NAME",
    "ROOT_MODULE_SENTINEL_TAG",
    "DEFAULT_EXCLUDE_PATHS",
]
