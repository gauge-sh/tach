import pytest
import tempfile
import shutil
import os
from modguard import errors
from modguard.init import init_project
from modguard.parsing.boundary import BOUNDARY_PRELUDE

@pytest.fixture(scope="module")
def test_root():
    # Create a temporary directory to use as the root for testing
    test_root = tempfile.mkdtemp()
    yield test_root
    # Remove the temporary directory after testing
    shutil.rmtree(test_root)

def test_init_project_with_valid_root(test_root):
    # Create some mock files and directories for testing
    test_dirs = [
        "package1",
        "package2",
        "package3",
        "package4/subpackage",
        "package5/subpackage",
    ]
    for d in test_dirs:
        os.makedirs(os.path.join(test_root, d))
        with open(os.path.join(test_root, d, "__init__.py"), "w") as f:
            f.write("# Mock __init__.py file")

    # Call init_project with the test root
    init_project(test_root)

    # Check if __init__.py files have been modified as expected
    for d in test_dirs:
        with open(os.path.join(test_root, d, "__init__.py")) as f:
            content = f.read()
            assert BOUNDARY_PRELUDE in content


def test_init_project_with_invalid_root():
    with pytest.raises(errors.ModguardSetupError):
        init_project("nonexistent_directory")