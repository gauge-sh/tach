import pytest
from unittest.mock import patch, MagicMock

from modguard.constants import CONFIG_FILE_NAME
from modguard.errors import ModguardError
from modguard.filesystem.module import validate_path_for_add


def test_path_does_not_exist():
    with pytest.raises(ModguardError) as excinfo:
        validate_path_for_add("/non/existent/path")
    assert "/non/existent/path does not exist." in str(excinfo.value)


def test_path_is_directory_with_module_file():
    with patch(
        "modguard.filesystem.module.os.path.exists",
        MagicMock(side_effect=lambda x: x.endswith(".yml") or x.endswith("dir")),
    ), patch("modguard.filesystem.module.os.path.isdir", return_value=True):
        with pytest.raises(ModguardError) as excinfo:
            validate_path_for_add("/some/dir")
    assert "/some/dir already contains a module.yml" in str(excinfo.value)


def test_directory_without_init_py():
    def mock_exists(path):
        return path.endswith("dir") and not path.endswith("__init__.py")

    with patch("modguard.filesystem.module.os.path.exists", mock_exists), patch(
        "modguard.filesystem.module.os.path.isdir", return_value=True
    ):
        with pytest.raises(ModguardError) as excinfo:
            validate_path_for_add("/some/dir")
    assert "/some/dir is not a valid Python package (no __init__.py found)." in str(
        excinfo.value
    )


def test_valid_directory():
    def mock_exists(path):
        return (
            not path.endswith("yml")
            and not path.endswith("yaml")
            or CONFIG_FILE_NAME in path
        )  # Everything exists for this test

    def mock_validate_project_config_path(path):
        return  # Assume validation is successful

    with patch("modguard.filesystem.module.os.path.exists", mock_exists), patch(
        "modguard.filesystem.validate_project_config_path",
        mock_validate_project_config_path,
    ), patch("modguard.filesystem.module.os.path.isdir", return_value=True):
        # No exception should be raised
        validate_path_for_add("/some/dir")


def test_non_python_file():
    with patch("modguard.filesystem.module.os.path.exists", return_value=True):
        with pytest.raises(ModguardError) as excinfo:
            validate_path_for_add("/some/file.txt")
    assert "/some/file.txt is not a Python file." in str(excinfo.value)


def test_python_file_with_matching_directory():
    def mock_exists(path):
        if path.endswith(".py"):
            return True
        return path == "/some/file"

    with patch("modguard.filesystem.module.os.path.exists", mock_exists):
        with pytest.raises(ModguardError) as excinfo:
            validate_path_for_add("/some/file.py")
    assert "{path} already has a directory of the same name." in str(excinfo.value)


def test_valid_python_file():
    def mock_exists(path):
        return path.endswith(".py")

    with patch(
        "modguard.filesystem.module.os.path.exists", side_effect=mock_exists
    ), patch(
        "modguard.filesystem.validate_project_config_path", side_effect=SystemError
    ):
        with pytest.raises(ModguardError) as excinfo:
            validate_path_for_add("/some/file.py")
    assert f"{CONFIG_FILE_NAME} does not exist in any parent directories" in str(
        excinfo.value
    )
