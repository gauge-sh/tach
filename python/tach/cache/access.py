from __future__ import annotations

import json
import uuid
from urllib import error, request

from tach.cache.setup import resolve_dot_tach
from tach.filesystem import find_project_config_root

# TODO: don't call find_project_config_root separately in each function, pass as argument


def get_uid() -> uuid.UUID | None:
    project_path = find_project_config_root()
    if project_path is None:
        return
    info_path = project_path / ".tach" / "tach.info"
    if not info_path.exists():
        resolve_dot_tach()
    contents = info_path.read_text().strip()
    uid = uuid.UUID(contents)
    return uid


def get_latest_version() -> str | None:
    project_path = find_project_config_root()
    if project_path is None:
        return
    latest_version_path = project_path / ".tach" / ".latest-version"
    if not latest_version_path.exists():
        return
    version = latest_version_path.read_text().strip()
    return version


def update_latest_version() -> None:
    project_path = find_project_config_root()
    if project_path is None:
        return
    url = "https://pypi.org/pypi/tach/json"
    try:
        # Sending a GET request to the URL
        with request.urlopen(url, timeout=1) as response:
            if response.status == 200:
                data = response.read().decode()
                json_data = json.loads(data)
                latest_version = json_data["info"]["version"]
            else:
                return
    except (error.URLError, KeyError):
        return
    (project_path / ".tach" / ".latest-version").write_text(latest_version)
