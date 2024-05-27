from __future__ import annotations

import os
import shutil
import tempfile

import pytest

from tach import errors
from tach import filesystem as fs
from tach.pkg import pkg_edit_interactive


def init_project_from_root(root) -> None:
    # Save the current working directory
    saved_directory = os.getcwd()
    try:
        # Navigate to the root directory and call init_project
        fs.chdir(root)
        pkg_edit_interactive(root)
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
        pkg_edit_interactive("nonexistent_directory")
