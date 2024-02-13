from unittest.mock import Mock
import pytest

from modguard import cli
from modguard.check import ErrorInfo


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


def test_execute_with_valid_dir(capfd, mock_isdir, mock_check):
    # Test with a valid path as mocked
    args = cli.parse_arguments(["check", "valid_dir"])
    with pytest.raises(SystemExit) as sys_exit:
        cli.modguard_check(args)
    captured = capfd.readouterr()
    assert sys_exit.value.code == 0
    assert "✅" in captured.out
    assert "All modules safely guarded!" in captured.out


def test_execute_with_error(capfd, mock_isdir, mock_check):
    # Test with a valid path as mocked
    args = cli.parse_arguments(["check", "valid_dir"])
    # Mock an error returned from check
    location = "valid_dir/file.py"
    message = "Import valid_dir in valid_dir/file.py is blocked by boundary"
    mock_check.return_value = [
        ErrorInfo(
            exception_message="Import valid_dir in valid_dir/file.py is blocked by boundary",
        )
    ]
    with pytest.raises(SystemExit) as sys_exit:
        cli.modguard_check(args)
    captured = capfd.readouterr()
    assert sys_exit.value.code == 1
    assert location in captured.err
    assert message in captured.err


def test_execute_with_invalid_dir(capfd, mock_isdir):
    with pytest.raises(SystemExit) as sys_exit:
        # Test with an invalid path as mocked
        args = cli.parse_arguments(["check", "invalid_dir"])
        cli.modguard_check(args)
    captured = capfd.readouterr()
    assert sys_exit.value.code == 1
    assert "invalid_dir is not a valid directory" in captured.err


def test_execute_with_valid_exclude(capfd, mock_isdir, mock_check):
    with pytest.raises(SystemExit) as sys_exit:
        # Test with a valid path as mocked
        args = cli.parse_arguments(["check", "valid_dir", "--exclude", "valid_dir"])
        cli.modguard_check(args)
    captured = capfd.readouterr()
    assert sys_exit.value.code == 0
    assert "✅" in captured.out
    assert "All modules safely guarded!" in captured.out


def test_execute_with_invalid_exclude(capfd, mock_isdir):
    with pytest.raises(SystemExit) as sys_exit:
        # Test with a valid path as mocked
        # Mock a valid return from check
        args = cli.parse_arguments(["check", "valid_dir", "--exclude", "invalid_dir"])
        cli.modguard_check(args)
    captured = capfd.readouterr()
    assert sys_exit.value.code == 1
    assert "invalid_dir is not a valid dir or file" in captured.err
