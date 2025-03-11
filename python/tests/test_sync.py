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
                add=True,
            )

        assert exc_info.value.code == 0
        captured = capfd.readouterr()
        assert "✅" in captured.err  # success state


def test_distributed_config_dir(example_dir, capfd):
    project_root = example_dir / "distributed_config"

    with tempfile.TemporaryDirectory() as temp_dir:
        temp_project_root = Path(temp_dir) / "distributed_config"
        shutil.copytree(project_root, temp_project_root)

        project_config = parse_project_config(root=temp_project_root)
        assert project_config is not None

        modules = project_config.all_modules()
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
            )

        assert exc_info.value.code == 0
        captured = capfd.readouterr()
        assert "✅" in captured.err  # success state

        project_config = parse_project_config(root=temp_project_root)
        assert project_config is not None

        modules = project_config.all_modules()
        assert len(modules) == 3

        top_level_module = next(
            module for module in modules if module.path == "project.top_level"
        )
        assert set(map(lambda dep: dep.path, top_level_module.depends_on)) == {
            "project.module_two",
            "project.module_one",
        }


def test_many_features_example_dir(example_dir, capfd):
    project_root = example_dir / "many_features"

    with tempfile.TemporaryDirectory() as temp_dir:
        temp_project_root = Path(temp_dir) / "many_features"
        shutil.copytree(project_root, temp_project_root)

        project_config = parse_project_config(root=temp_project_root)
        assert project_config is not None

        with pytest.raises(SystemExit) as exc_info:
            tach_sync(
                project_root=temp_project_root,
                project_config=project_config,
            )

        assert exc_info.value.code == 0
        captured = capfd.readouterr()
        assert "✅" in captured.err  # success state

        project_config = parse_project_config(root=temp_project_root)
        assert project_config is not None

        modules = project_config.all_modules()
        # This should be the number of statically defined modules, before globbing
        assert len(modules) == 15

        module2 = next(module for module in modules if module.path == "module2")
        assert set(map(lambda dep: dep.path, module2.depends_on)) == {"outer_module"}

        module3 = next(module for module in modules if module.path == "module3")
        assert set(map(lambda dep: dep.path, module3.depends_on)) == {"module1"}

        assert (
            '"//module1"'
            in (
                temp_project_root / "real_src" / "module3" / "tach.domain.toml"
            ).read_text()
        )

        assert (
            '"<domain_root>"'
            in (
                temp_project_root / "real_src" / "module3" / "tach.domain.toml"
            ).read_text()
        )
