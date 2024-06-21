from __future__ import annotations

import json
import uuid
from unittest.mock import MagicMock, patch
from urllib.error import URLError

from tach.cache.access import get_latest_version, get_uid, update_latest_version
from tach.cache.setup import resolve_dot_tach


@patch("tach.cache.setup.find_project_config_root")
def test_resolve_dot_tach(mock_find_project_config_root, tmp_path):
    project_path = tmp_path / "project"
    project_path.mkdir(parents=True, exist_ok=True)
    mock_find_project_config_root.return_value = project_path
    version = "1.0.0"

    with patch("tach.cache.setup.__version__", version):
        result = resolve_dot_tach()

    tach_path = project_path / ".tach"
    assert tach_path.exists()
    assert (tach_path / "tach.info").exists()
    assert (tach_path / "tach.info").read_text().strip() != ""
    assert (tach_path / ".gitignore").exists()
    assert (tach_path / ".latest-version").exists()
    assert (tach_path / ".latest-version").read_text().strip() == version
    assert result == tach_path


@patch("tach.cache.access.find_project_config_root")
@patch("tach.cache.access.resolve_dot_tach")
def test_get_uid(mock_resolve_dot_tach, mock_find_project_config_root, tmp_path):
    project_path = tmp_path / "project"
    tach_info_path = project_path / ".tach" / "tach.info"
    tach_info_path.parent.mkdir(parents=True, exist_ok=True)
    uid = uuid.uuid4()
    tach_info_path.write_text(str(uid))

    mock_find_project_config_root.return_value = project_path

    result = get_uid()
    assert result == uid


@patch("tach.cache.access.find_project_config_root")
def test_get_uid_no_project_path(mock_find_project_config_root):
    mock_find_project_config_root.return_value = None
    result = get_uid()
    assert result is None


@patch("tach.cache.access.find_project_config_root")
def test_get_latest_version(mock_find_project_config_root, tmp_path):
    project_path = tmp_path / "project"
    latest_version_path = project_path / ".tach" / ".latest-version"
    latest_version_path.parent.mkdir(parents=True, exist_ok=True)
    version = "1.0.0"
    latest_version_path.write_text(version)

    mock_find_project_config_root.return_value = project_path

    result = get_latest_version()
    assert result == version


@patch("tach.cache.access.find_project_config_root")
def test_update_latest_version(mock_find_project_config_root, tmp_path):
    project_path = tmp_path / "project"
    latest_version_path = project_path / ".tach" / ".latest-version"
    latest_version_path.parent.mkdir(parents=True, exist_ok=True)

    mock_find_project_config_root.return_value = project_path

    mock_response = MagicMock()
    mock_response.status = 200
    mock_response.read.return_value = json.dumps(
        {"info": {"version": "1.0.1"}}
    ).encode()
    mock_urlopen = MagicMock()
    mock_urlopen.return_value.__enter__.return_value = mock_response

    with patch("tach.cache.access.request.urlopen", mock_urlopen):
        update_latest_version()

    assert latest_version_path.read_text().strip() == "1.0.1"


@patch("tach.cache.access.find_project_config_root")
@patch("tach.cache.access.request.urlopen")
def test_update_latest_version_network_error(
    mock_urlopen, mock_find_project_config_root, tmp_path
):
    project_path = tmp_path / "project"
    latest_version_path = project_path / ".tach" / ".latest-version"
    latest_version_path.parent.mkdir(parents=True, exist_ok=True)

    mock_find_project_config_root.return_value = project_path
    mock_urlopen.side_effect = URLError("Network error")

    update_latest_version()
    assert not latest_version_path.exists()
