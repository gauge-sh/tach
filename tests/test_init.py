import pytest
import tempfile
import shutil
import os
from tach import errors, filesystem as fs
from tach.init import init_project


def init_project_from_root(root) -> None:
    # Save the current working directory
    saved_directory = os.getcwd()
    try:
        # Navigate to the root directory and call init_project
        fs.chdir(root)
        init_project(root)
    finally:
        # Change back to the original directory
        fs.chdir(saved_directory)


@pytest.fixture(scope="module")
def test_root():
    # Create a temporary directory to use as the root for testing
    test_root = tempfile.mkdtemp()
    yield test_root
    # Remove the temporary directory after testing
    shutil.rmtree(test_root)


def test_init_project_with_invalid_root():
    with pytest.raises(errors.TachSetupError):
        init_project("nonexistent_directory")
