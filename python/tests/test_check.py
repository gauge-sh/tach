from __future__ import annotations

import json
from unittest.mock import NonCallableMagicMock

import pytest

from tach.cli import tach_check
from tach.errors import TachCircularDependencyError, TachVisibilityError
from tach.extension import CheckDiagnostics
from tach.icons import SUCCESS, WARNING
from tach.parsing.config import parse_project_config


def test_valid_example_dir(example_dir, capfd):
    project_root = example_dir / "valid"
    project_config = parse_project_config(root=project_root)
    assert project_config is not None
    with pytest.raises(SystemExit) as exc_info:
        tach_check(
            project_root=project_root,
            project_config=project_config,
            exclude_paths=project_config.exclude,
        )
    assert exc_info.value.code == 0
    captured = capfd.readouterr()
    assert SUCCESS in captured.out
    assert WARNING in captured.err


def test_valid_example_dir_monorepo(example_dir):
    project_root = example_dir / "monorepo"
    project_config = parse_project_config(root=project_root)
    assert project_config is not None
    with pytest.raises(SystemExit) as exc_info:
        tach_check(
            project_root=project_root,
            project_config=project_config,
            exclude_paths=project_config.exclude,
        )
    assert exc_info.value.code == 0


def test_check_json_output(example_dir, capfd, mocker):
    project_root = example_dir / "valid"
    project_config = parse_project_config(root=project_root)
    assert project_config is not None

    mock_diagnostics = NonCallableMagicMock(spec=CheckDiagnostics)
    mock_diagnostics.serialize_json.return_value = json.dumps(
        {"errors": [], "warnings": []}
    )
    mocker.patch("tach.cli.check", return_value=mock_diagnostics)

    with pytest.raises(SystemExit) as exc_info:
        tach_check(
            project_root=project_root,
            project_config=project_config,
            exclude_paths=project_config.exclude,
            output_format="json",
        )
    assert exc_info.value.code == 0

    captured = capfd.readouterr()
    assert json.loads(captured.out) == {"errors": [], "warnings": []}


def test_check_json_with_errors(example_dir, capfd, mocker):
    project_root = example_dir / "valid"
    project_config = parse_project_config(root=project_root)
    assert project_config is not None

    mock_diagnostics = NonCallableMagicMock(spec=CheckDiagnostics)
    mock_diagnostics.serialize_json.return_value = json.dumps(
        {"errors": ["error1", "error2"], "warnings": ["warning1"]}
    )
    mocker.patch("tach.cli.check", return_value=mock_diagnostics)

    with pytest.raises(SystemExit):
        tach_check(
            project_root=project_root,
            project_config=project_config,
            exclude_paths=project_config.exclude,
            output_format="json",
        )

    captured = capfd.readouterr()
    assert json.loads(captured.out) == {
        "errors": ["error1", "error2"],
        "warnings": ["warning1"],
    }


def test_check_circular_dependency_text(example_dir, capfd, mocker):
    project_root = example_dir / "valid"
    project_config = parse_project_config(root=project_root)
    assert project_config is not None

    mocker.patch(
        "tach.cli.check",
        side_effect=TachCircularDependencyError(["mod1", "mod2", "mod1"]),
    )

    with pytest.raises(SystemExit) as exc_info:
        tach_check(
            project_root=project_root,
            project_config=project_config,
            exclude_paths=project_config.exclude,
        )
    assert exc_info.value.code == 1

    captured = capfd.readouterr()
    assert "Circular dependency detected" in captured.err
    assert "'mod1'" in captured.err
    assert "'mod2'" in captured.err


def test_check_circular_dependency_json(example_dir, capfd, mocker):
    project_root = example_dir / "valid"
    project_config = parse_project_config(root=project_root)
    assert project_config is not None

    mocker.patch(
        "tach.cli.check",
        side_effect=TachCircularDependencyError(["mod1", "mod2", "mod1"]),
    )

    with pytest.raises(SystemExit) as exc_info:
        tach_check(
            project_root=project_root,
            project_config=project_config,
            exclude_paths=project_config.exclude,
            output_format="json",
        )
    assert exc_info.value.code == 1

    captured = capfd.readouterr()
    result = json.loads(captured.out)
    assert result["error"] == "Circular dependency"
    assert result["dependencies"] == ["mod1", "mod2", "mod1"]


def test_check_visibility_error_text(example_dir, capfd, mocker):
    project_root = example_dir / "valid"
    project_config = parse_project_config(root=project_root)
    assert project_config is not None

    visibility_errors = [("mod1", "mod2", ["public"])]
    mocker.patch("tach.cli.check", side_effect=TachVisibilityError(visibility_errors))

    with pytest.raises(SystemExit) as exc_info:
        tach_check(
            project_root=project_root,
            project_config=project_config,
            exclude_paths=project_config.exclude,
        )
    assert exc_info.value.code == 1

    captured = capfd.readouterr()
    assert "Module configuration error" in captured.err
    assert "'mod1' cannot depend on 'mod2'" in captured.err
    assert "public" in captured.err


def test_check_visibility_error_json(example_dir, capfd, mocker):
    project_root = example_dir / "valid"
    project_config = parse_project_config(root=project_root)
    assert project_config is not None

    visibility_errors = [("mod1", "mod2", ["public"])]
    mocker.patch("tach.cli.check", side_effect=TachVisibilityError(visibility_errors))

    with pytest.raises(SystemExit) as exc_info:
        tach_check(
            project_root=project_root,
            project_config=project_config,
            exclude_paths=project_config.exclude,
            output_format="json",
        )
    assert exc_info.value.code == 1

    captured = capfd.readouterr()
    result = json.loads(captured.out)
    assert result["error"] == "Visibility error"
    assert result["visibility_errors"] == [["mod1", "mod2", ["public"]]]
