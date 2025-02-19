from __future__ import annotations

import uuid
from unittest.mock import patch

from tach.cache.access import get_latest_version, get_uid
from tach.cache.setup import resolve_dot_tach


def test_resolve_dot_tach(tmp_path):
    project_path = tmp_path / "project"
    project_path.mkdir(parents=True, exist_ok=True)
    version = "1.0.0"

    with patch("tach.cache.setup.__version__", version):
        result = resolve_dot_tach(project_path)

    tach_path = project_path / ".tach"
    assert tach_path.exists()
    assert (tach_path / "tach.info").exists()
    assert (tach_path / "tach.info").read_text().strip() != ""
    assert (tach_path / ".gitignore").exists()
    assert (tach_path / ".latest-version").exists()
    assert (tach_path / ".latest-version").read_text().strip() == version
    assert result == tach_path


@patch("tach.cache.access.resolve_dot_tach")
def test_get_uid(mock_resolve_dot_tach, tmp_path):
    project_path = tmp_path / "project"
    tach_info_path = project_path / ".tach" / "tach.info"
    tach_info_path.parent.mkdir(parents=True, exist_ok=True)
    uid = uuid.uuid4()
    tach_info_path.write_text(str(uid))

    result = get_uid(project_path)
    assert result == uid


def test_get_latest_version(tmp_path):
    project_path = tmp_path / "project"
    latest_version_path = project_path / ".tach" / ".latest-version"
    latest_version_path.parent.mkdir(parents=True, exist_ok=True)
    version = "1.0.0"
    latest_version_path.write_text(version)

    result = get_latest_version(project_path)
    assert result == version
