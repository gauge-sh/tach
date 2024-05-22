from __future__ import annotations

from unittest.mock import Mock

import pytest

from tach import cli
from tach.check import BoundaryError, ErrorInfo
from tach.constants import CONFIG_FILE_NAME
from tach.core import ProjectConfig, TagDependencyRules


@pytest.fixture
def mock_check(mocker) -> Mock:
    mock = Mock(return_value=[])  # default to a return with no errors
    mocker.patch("tach.cli.check", mock)
    return mock


@pytest.fixture
def mock_isdir(mocker) -> None:
    def mock_isdir(path: str) -> bool:
        if path == "valid_dir":
            return True
        else:
            return False

    mocker.patch("tach.filesystem.project.os.path.isdir", mock_isdir)


@pytest.fixture
def mock_path_exists(mocker) -> None:
    def mock_path_exists(path: str) -> bool:
        if CONFIG_FILE_NAME in path:
            return True
        else:
            return False

    mocker.patch("tach.filesystem.project.os.path.exists", mock_path_exists)


@pytest.fixture
def mock_project_config(mocker) -> None:
    def mock_project_config(root: str = "") -> ProjectConfig:
        return ProjectConfig(
            constraints=[TagDependencyRules(tag="mocked", depends_on=["mocked"])]
        )

    mocker.patch("tach.cli.parse_project_config", mock_project_config)


def test_execute_with_tach_yml(
    capfd, mock_path_exists, mock_check, mock_project_config
):
    # Test with a valid path as mocked
    args, _ = cli.parse_arguments(["check"])
    assert args.command == "check"
    with pytest.raises(SystemExit) as sys_exit:
        cli.tach_check()
    captured = capfd.readouterr()
    assert sys_exit.value.code == 0
    assert "✅" in captured.out
    assert "All package dependencies validated!" in captured.out


def test_execute_with_error(capfd, mock_path_exists, mock_check, mock_project_config):
    # Mock an error returned from check
    location = "valid_dir/file.py"
    message = "Import valid_dir in valid_dir/file.py is blocked by boundary"
    mock_check.return_value = [
        BoundaryError(
            file_path=location,
            line_number=0,
            import_mod_path="valid_dir",
            error_info=ErrorInfo(
                exception_message="Import valid_dir in valid_dir/file.py is blocked by boundary",
            ),
        )
    ]
    with pytest.raises(SystemExit) as sys_exit:
        cli.tach_check()
    captured = capfd.readouterr()
    assert sys_exit.value.code == 1
    assert location in captured.err
    assert message in captured.err


def test_invalid_command(capfd):
    with pytest.raises(SystemExit) as sys_exit:
        # Test with an invalid command
        cli.parse_arguments(["help"])
    captured = capfd.readouterr()
    assert sys_exit.value.code == 2
    assert "invalid choice: 'help" in captured.err


def test_execute_with_valid_exclude(
    capfd, mock_isdir, mock_path_exists, mock_check, mock_project_config
):
    with pytest.raises(SystemExit) as sys_exit:
        # Test with a valid path as mocked
        args, _ = cli.parse_arguments(["check", "--exclude", "valid_dir"])
        exclude_paths = args.exclude.split(",")
        cli.tach_check(exclude_paths=exclude_paths)
    captured = capfd.readouterr()
    assert sys_exit.value.code == 0
    assert "✅" in captured.out
    assert "All package dependencies validated!" in captured.out
