from __future__ import annotations

import pathlib
from pathlib import Path
from unittest.mock import Mock

import pytest

from tach import cli
from tach.extension import ProjectConfig

_VALID_TACH_TOML = pathlib.Path(__file__).parent / "example" / "valid" / "tach.toml"


@pytest.fixture
def mock_check(mocker) -> Mock:
    mock = Mock(return_value=[])  # default to a return with no errors
    mocker.patch("tach.extension.check", mock)
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
        )
    captured = capfd.readouterr()
    assert sys_exit.value.code == 0
    assert "✅" in captured.err
    assert "All modules validated!" in captured.err


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
        mock_project_config.exclude = args.exclude.split(",")
        cli.tach_check(
            project_root=Path(),
            project_config=mock_project_config,
        )
    captured = capfd.readouterr()
    assert sys_exit.value.code == 0
    assert "✅" in captured.err
    assert "All modules validated!" in captured.err


def test_tach_server_with_config(tmp_path, mocker):
    mock_run_server = mocker.patch("tach.extension.run_server", autospec=True)

    cli.main(["server", "--config", str(_VALID_TACH_TOML)])
    # Verify server was run with the custom config.
    mock_run_server.assert_called_once()
    assert "domain_four.py" in mock_run_server.call_args[0][1].exclude

    mock_run_server.reset_mock()

    # Should still work even if it's not named tach.toml
    toml_contents = _VALID_TACH_TOML.read_text()
    custom_config_path = tmp_path.joinpath("custom_config.toml")
    custom_config_path.write_text(toml_contents)
    cli.main(["server", "--config", str(custom_config_path)])

    # Verify server was run with the custom config.
    mock_run_server.assert_called_once()
    assert "domain_four.py" in mock_run_server.call_args[0][1].exclude
