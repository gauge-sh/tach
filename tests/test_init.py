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


def test_init_project_with_valid_root(test_root):
    # Create some mock files and directories for testing
    test_dirs = [
        "package1",
        "package2",
        "package3",
        "package4/subpackage",
        "package5/subpackage",
        "package6/subpackage",
    ]
    for d in test_dirs:
        os.makedirs(os.path.join(test_root, d))
        with open(os.path.join(test_root, d, "__init__.py"), "w") as f:
            f.write("# Mock __init__.py file")

    # Create some mock Python files with imports and member names
    file_contents = {
        "package1/__init__.py": "from package6.subpackage.module6 import x",
        "package2/__init__.py": "",
        "package1/module1.py": "class Package1Class:\n    pass\n",
        "package2/module2.py": "def package_2_func():\n    pass\n",
        "package3/__init__.py": "from package1.module1 import Package1Class\nfrom package2.module2 import package_2_func\n",
        "package3/module3.py": "",
        "package4/subpackage/__init__.py": "",
        "package5/subpackage/__init__.py": "import package3.module3",
        "package6/subpackage/__init__.py": "",
        "package6/subpackage/module6.py": "x = 3\n",
    }

    for file_path, content in file_contents.items():
        with open(os.path.join(test_root, file_path), "w") as f:
            f.write(content)

    # Call init_project with the test root
    init_project_from_root(test_root)

    # TODO: test new behavior


def test_init_project_with_invalid_root():
    with pytest.raises(errors.TachSetupError):
        init_project("nonexistent_directory")
