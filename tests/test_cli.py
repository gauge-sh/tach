from unittest.mock import Mock
import pytest

from modguard import cli
from modguard.check import ErrorInfo
from modguard.constants import CONFIG_FILE_NAME


@pytest.fixture
def mock_check(mocker) -> Mock:
    mock = Mock(return_value=[])  # default to a return with no errors
    mocker.patch("modguard.cli.check", mock)
    return mock


@pytest.fixture
def mock_isdir(mocker) -> None:
    def mock_isdir(path: str) -> bool:
        if path == "valid_dir":
            return True
        else:
            return False

    mocker.patch("modguard.cli.os.path.isdir", mock_isdir)


@pytest.fixture
def mock_path_exists(mocker) -> None:
    def mock_path_exists(path: str) -> bool:
        if CONFIG_FILE_NAME in path:
            return True
        else:
            return False

    mocker.patch("modguard.filesystem.project.os.path.exists", mock_path_exists)


def test_execute_with_modguard_yml(capfd, mock_path_exists, mock_check):
    # Test with a valid path as mocked
    args = cli.parse_arguments(["check"])
    assert args.command == "check"
    with pytest.raises(SystemExit) as sys_exit:
        cli.modguard_check()
    captured = capfd.readouterr()
    assert sys_exit.value.code == 0
    assert "✅" in captured.out
    assert "All modules safely guarded!" in captured.out


def test_execute_with_error(capfd, mock_path_exists, mock_check):
    # Mock an error returned from check
    location = "valid_dir/file.py"
    message = "Import valid_dir in valid_dir/file.py is blocked by boundary"
    mock_check.return_value = [
        ErrorInfo(
            exception_message="Import valid_dir in valid_dir/file.py is blocked by boundary",
        )
    ]
    with pytest.raises(SystemExit) as sys_exit:
        cli.modguard_check()
    captured = capfd.readouterr()
    assert sys_exit.value.code == 1
    assert location in captured.err
    assert message in captured.err


def test_execute_with_no_modguard_yml(capfd):
    with pytest.raises(SystemExit) as sys_exit:
        # Test with no modguard.yml mocked
        cli.parse_arguments(["check"])
    captured = capfd.readouterr()
    assert sys_exit.value.code == 1
    assert "modguard.(yml|yaml) not found" in captured.err


def test_show_with_no_modguard_yml(capfd):
    with pytest.raises(SystemExit) as sys_exit:
        # Test with no modguard.yml mocked
        cli.parse_arguments(["show"])
    captured = capfd.readouterr()
    assert sys_exit.value.code == 1
    assert "modguard.(yml|yaml) not found" in captured.err


def test_invalid_command(capfd):
    with pytest.raises(SystemExit) as sys_exit:
        # Test with an invalid command
        cli.parse_arguments(["help"])
    captured = capfd.readouterr()
    assert sys_exit.value.code == 2
    assert "invalid choice: 'help" in captured.err


def test_execute_with_valid_exclude(capfd, mock_isdir, mock_path_exists, mock_check):
    with pytest.raises(SystemExit) as sys_exit:
        # Test with a valid path as mocked
        args = cli.parse_arguments(["check", "--exclude", "valid_dir"])
        exclude_paths = args.exclude.split(",")
        cli.modguard_check(exclude_paths=exclude_paths)
    captured = capfd.readouterr()
    assert sys_exit.value.code == 0
    assert "✅" in captured.out
    assert "All modules safely guarded!" in captured.out


def test_execute_with_invalid_exclude(capfd, mock_isdir, mock_path_exists):
    with pytest.raises(SystemExit) as sys_exit:
        # Test with a valid path as mocked
        # Mock a valid return from check
        args = cli.parse_arguments(["check", "--exclude", "invalid_dir"])
        exclude_paths = args.exclude.split(",")
        cli.modguard_check(exclude_paths=exclude_paths)
    captured = capfd.readouterr()
    assert sys_exit.value.code == 1
    assert "invalid_dir is not a valid dir or file" in captured.err
