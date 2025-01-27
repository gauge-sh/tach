from __future__ import annotations

from pathlib import Path
from unittest.mock import Mock

import pytest

from tach import cli
from tach.extension import ProjectConfig


@pytest.fixture
def mock_check(mocker) -> Mock:
    mock = Mock(return_value=[])  # default to a return with no errors
    mocker.patch("tach.cli.check", mock)
    return mock


@pytest.fixture
def mock_project_config(mocker) -> ProjectConfig:
    def _mock_project_config(root: str = "") -> ProjectConfig:
        return ProjectConfig()

    mocker.patch("tach.cli.parse_project_config", _mock_project_config)
    return _mock_project_config()


def test_execute_with_config(capfd, mock_check, mock_project_config):
    # Test with a valid path as mocked
    args, _ = cli.parse_arguments(["check"])
    assert args.command == "check"
    with pytest.raises(SystemExit) as sys_exit:
        cli.tach_check(
            project_root=Path(),
            project_config=mock_project_config,
            exclude_paths=mock_project_config.exclude,
        )
    captured = capfd.readouterr()
    assert sys_exit.value.code == 0
    assert "✅" in captured.out
    assert "All modules validated!" in captured.out


def test_invalid_command(capfd):
    with pytest.raises(SystemExit) as sys_exit:
        # Test with an invalid command
        cli.parse_arguments(["help"])
    captured = capfd.readouterr()
    assert sys_exit.value.code == 2
    assert "invalid choice: 'help" in captured.err


def test_execute_with_valid_exclude(capfd, mock_check, mock_project_config):
    with pytest.raises(SystemExit) as sys_exit:
        # Test with a valid path as mocked
        args, _ = cli.parse_arguments(["check", "--exclude", "valid_dir"])
        exclude_paths = args.exclude.split(",")
        cli.tach_check(
            project_root=Path(),
            project_config=mock_project_config,
            exclude_paths=exclude_paths,
        )
    captured = capfd.readouterr()
    assert sys_exit.value.code == 0
    assert "✅" in captured.out
    assert "All modules validated!" in captured.out
