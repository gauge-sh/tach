from __future__ import annotations

import shutil
import tempfile
from pathlib import Path

import pytest

from tach.cli import tach_sync
from tach.parsing.config import parse_project_config


def test_valid_example_dir(example_dir, capfd):
    project_root = example_dir / "valid"

    with tempfile.TemporaryDirectory() as temp_dir:
        temp_project_root = Path(temp_dir) / "valid"
        shutil.copytree(project_root, temp_project_root)

        project_config = parse_project_config(root=temp_project_root)
        assert project_config is not None

        with pytest.raises(SystemExit) as exc_info:
            tach_sync(
                project_root=temp_project_root,
                project_config=project_config,
                exclude_paths=project_config.exclude,
                add=True,
            )

        assert exc_info.value.code == 0
        captured = capfd.readouterr()
        assert "✅" in captured.out  # success state


def test_distributed_config_dir(example_dir, capfd):
    project_root = example_dir / "distributed_config"

    with tempfile.TemporaryDirectory() as temp_dir:
        temp_project_root = Path(temp_dir) / "distributed_config"
        shutil.copytree(project_root, temp_project_root)

        project_config = parse_project_config(root=temp_project_root)
        assert project_config is not None

        modules = project_config.filtered_modules([])
        assert len(modules) == 3

        top_level_module = next(
            module for module in modules if module.path == "project.top_level"
        )
        assert set(map(lambda dep: dep.path, top_level_module.depends_on)) == {
            "project.module_two"
        }

        with pytest.raises(SystemExit) as exc_info:
            tach_sync(
                project_root=temp_project_root,
                project_config=project_config,
                exclude_paths=project_config.exclude,
            )

        assert exc_info.value.code == 0
        captured = capfd.readouterr()
        print(captured.err)
        print(captured.out)
        assert "✅" in captured.out  # success state

        project_config = parse_project_config(root=temp_project_root)
        print(project_config.serialize_json())
        assert project_config is not None

        modules = project_config.filtered_modules([])
        assert len(modules) == 3

        top_level_module = next(
            module for module in modules if module.path == "project.top_level"
        )
        assert set(map(lambda dep: dep.path, top_level_module.depends_on)) == {
            "project.module_two",
            "project.module_one",
        }
