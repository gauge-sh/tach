from __future__ import annotations

import json
import uuid
from urllib import error, request

from tach.cache.setup import get_project_path, resolve_dot_tach


def get_uid() -> uuid.UUID | None:
    project_path = get_project_path()
    if project_path is None:
        return
    if not (project_path / ".tach" / "tach.info").exists():
        resolve_dot_tach()
    contents = (project_path / ".tach" / "tach.info").read_text().strip()
    uid = uuid.UUID(contents)
    return uid


def get_latest_version() -> str | None:
    project_path = get_project_path()
    if project_path is None:
        return
    if not (project_path / ".tach" / ".latest-version").exists():
        return
    version = (project_path / ".tach" / ".latest-version").read_text().strip()
    return version


def update_latest_version() -> None:
    project_path = get_project_path()
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
