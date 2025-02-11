from __future__ import annotations

import json
from unittest.mock import NonCallableMagicMock

import pytest

from tach.cli import tach_check, tach_check_external
from tach.errors import TachCircularDependencyError, TachVisibilityError
from tach.extension import Diagnostic
from tach.icons import FAIL, SUCCESS, WARNING
from tach.parsing.config import parse_project_config


def test_valid_example_dir(example_dir, capfd):
    project_root = example_dir / "valid"
    project_config = parse_project_config(root=project_root)
    assert project_config is not None
    with pytest.raises(SystemExit) as exc_info:
        tach_check(
            project_root=project_root,
            project_config=project_config,
        )
    assert exc_info.value.code == 0
    captured = capfd.readouterr()
    assert SUCCESS in captured.out
    assert WARNING in captured.err or "WARN" in captured.err


def test_valid_example_dir_monorepo(example_dir):
    project_root = example_dir / "monorepo"
    project_config = parse_project_config(root=project_root)
    assert project_config is not None
    with pytest.raises(SystemExit) as exc_info:
        tach_check(
            project_root=project_root,
            project_config=project_config,
        )
    assert exc_info.value.code == 0


def test_check_json_output(example_dir, capfd, mocker):
    project_root = example_dir / "valid"
    project_config = parse_project_config(root=project_root)
    assert project_config is not None

    mock_diagnostics = [NonCallableMagicMock(spec=Diagnostic)]
    mock_diagnostics[0].is_error.return_value = False
    mocker.patch(
        "tach.extension.serialize_diagnostics_json",
        return_value=json.dumps([{"hello": "world"}]),
    )
    mocker.patch("tach.extension.check", return_value=mock_diagnostics)

    with pytest.raises(SystemExit) as exc_info:
        tach_check(
            project_root=project_root,
            project_config=project_config,
            output_format="json",
        )
    assert exc_info.value.code == 0

    captured = capfd.readouterr()
    assert json.loads(captured.out) == [{"hello": "world"}]


def test_check_json_with_errors(example_dir, capfd, mocker):
    project_root = example_dir / "valid"
    project_config = parse_project_config(root=project_root)
    assert project_config is not None

    mock_diagnostics = [NonCallableMagicMock(spec=Diagnostic)]
    mocker.patch(
        "tach.extension.serialize_diagnostics_json",
        return_value=json.dumps(
            {"errors": ["error1", "error2"], "warnings": ["warning1"]}
        ),
    )
    mocker.patch("tach.extension.check", return_value=mock_diagnostics)

    with pytest.raises(SystemExit):
        tach_check(
            project_root=project_root,
            project_config=project_config,
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
        "tach.extension.check",
        side_effect=TachCircularDependencyError(["mod1", "mod2", "mod1"]),
    )

    with pytest.raises(SystemExit) as exc_info:
        tach_check(
            project_root=project_root,
            project_config=project_config,
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
        "tach.extension.check",
        side_effect=TachCircularDependencyError(["mod1", "mod2", "mod1"]),
    )

    with pytest.raises(SystemExit) as exc_info:
        tach_check(
            project_root=project_root,
            project_config=project_config,
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
    mocker.patch(
        "tach.extension.check", side_effect=TachVisibilityError(visibility_errors)
    )

    with pytest.raises(SystemExit) as exc_info:
        tach_check(
            project_root=project_root,
            project_config=project_config,
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
    mocker.patch(
        "tach.extension.check", side_effect=TachVisibilityError(visibility_errors)
    )

    with pytest.raises(SystemExit) as exc_info:
        tach_check(
            project_root=project_root,
            project_config=project_config,
            output_format="json",
        )
    assert exc_info.value.code == 1

    captured = capfd.readouterr()
    result = json.loads(captured.out)
    assert result["error"] == "Visibility error"
    assert result["visibility_errors"] == [["mod1", "mod2", ["public"]]]


def test_distributed_config_example_dir(example_dir, capfd):
    project_root = example_dir / "distributed_config"
    project_config = parse_project_config(root=project_root)
    assert project_config is not None

    with pytest.raises(SystemExit) as exc_info:
        tach_check(
            project_root=project_root,
            project_config=project_config,
        )
    assert exc_info.value.code == 1

    captured = capfd.readouterr()
    assert FAIL in captured.err or "FAIL" in captured.err
    assert "Cannot use 'project.module_one.module_one'" in captured.err
    assert (
        "Module 'project.top_level' cannot depend on 'project.module_one'"
        in captured.err
    )
    assert "project/top_level.py" in captured.err


def _check_expected_messages(section_text: str, expected_messages: list[tuple]) -> None:
    """Helper to verify all expected messages appear in a section of output text, in the given order.

    Args:
        section_text: The text section to check
        expected_messages: List of tuples containing substrings that should appear together in a line
    """
    lines = iter(section_text.split("\n"))
    substrs = iter(expected_messages)
    current_substrs = next(substrs, None)

    for line in lines:
        if not current_substrs:
            break

        if all(substr.lower() in line.lower() for substr in current_substrs):
            current_substrs = next(substrs, None)

    assert current_substrs is None, (
        f"Not all expected messages were found: {list(substrs)} in section: {section_text}"
    )


def _check_expected_messages_unordered(
    section_text: str, expected_messages: list[tuple]
) -> None:
    """Helper to verify all expected messages appear in a section of output text.

    Args:
        section_text: The text section to check
        expected_messages: List of tuples containing substrings that should appear together in a line
    """
    lines = iter(section_text.split("\n"))
    substr_tuples = set(expected_messages)
    for line in lines:
        if "[WARN]" in line or "[FAIL]" in line:
            matched = False
            for substr_tuple in substr_tuples:
                if all(substr.lower() in line.lower() for substr in substr_tuple):
                    substr_tuples.remove(substr_tuple)
                    matched = True
                    break
            if not matched:
                assert False, f"Unexpected warning/error line: {line}"

    assert not substr_tuples, (
        f"Not all expected messages were found: {list(substr_tuples)} in section: {section_text}"
    )


def test_many_features_example_dir(example_dir, capfd):
    project_root = example_dir / "many_features"
    project_config = parse_project_config(root=project_root)
    assert project_config is not None

    with pytest.raises(SystemExit) as exc_info:
        tach_check(
            project_root=project_root,
            project_config=project_config,
        )
    assert exc_info.value.code == 1

    captured = capfd.readouterr()
    general_header = captured.err.index("General\n")
    interfaces_header = captured.err.index("Interfaces\n")
    dependencies_header = captured.err.index("Internal Dependencies\n")

    general_section = captured.err[general_header:interfaces_header]
    interfaces_section = captured.err[interfaces_header:dependencies_header]
    dependencies_section = captured.err[dependencies_header:]

    expected_general = [
        (
            "[WARN]",
            "other_src_root/module1/api.py",
            "ignore directive",
            "missing a reason",
        ),
        (
            "[WARN]",
            "other_src_root/module4/service.py",
            "ignore directive",
            "missing a reason",
        ),
        ("[WARN]", "real_src/main.py", "ignore directive", "missing a reason"),
        (
            "[FAIL]",
            "other_src_root/module4/service.py",
            "L6",
            "ignore directive",
            "unused",
        ),
        (
            "[FAIL]",
            "other_src_root/module4/service.py",
            "L12",
            "ignore directive",
            "unused",
        ),
        ("[FAIL]", "other_src_root/module1/api.py", "ignore directive", "unused"),
        ("[FAIL]", "real_src/main.py", "ignore directive", "unused"),
    ]

    expected_interfaces = [
        (
            "[FAIL]",
            "real_src/module1/__init__.py",
            "module3.anything",
            "public interface",
        ),
        (
            "[FAIL]",
            "real_src/module1/controller.py",
            "module5.something",
            "public interface",
        ),
        (
            "[FAIL]",
            "real_src/module1/controller.py",
            "module3.anything",
            "public interface",
        ),
    ]

    expected_dependencies = [
        ("[FAIL]", "real_src/module2/service.py", "outer_module", "module2"),
        ("[FAIL]", "real_src/module3/__init__.py", "'low'", "lower than", "'mid'"),
    ]

    _check_expected_messages_unordered(general_section, expected_general)
    _check_expected_messages_unordered(interfaces_section, expected_interfaces)
    _check_expected_messages_unordered(dependencies_section, expected_dependencies)


def test_many_features_example_dir__external(example_dir, capfd):
    project_root = example_dir / "many_features"
    project_config = parse_project_config(root=project_root)
    assert project_config is not None

    with pytest.raises(SystemExit) as exc_info:
        tach_check_external(
            project_root=project_root,
            project_config=project_config,
        )
    assert exc_info.value.code == 1

    captured = capfd.readouterr()
    general_header = captured.err.index("General\n")
    external_header = captured.err.index("External Dependencies\n")

    general_section = captured.err[general_header:external_header]
    external_section = captured.err[external_header:]

    expected_general = [
        ("[WARN]", "real_src/main.py", "ignore directive", "missing a reason"),
        (
            "[WARN]",
            "real_src/module1/__init__.py",
            "ignore directive",
            "missing a reason",
        ),
        ("[FAIL]", "real_src/module1/__init__.py", "ignore directive", "unused"),
    ]

    expected_external = [
        ("[FAIL]", "prompt_toolkit", "not used"),
        ("[FAIL]", "importlib_metadata", "not used"),
        ("[FAIL]", "tomli_w", "not used"),
        ("[FAIL]", "pydot", "not used"),
        ("[FAIL]", "rich", "not used"),
        ("[FAIL]", "networkx", "not used"),
        ("[FAIL]", "stdlib_list", "not used"),
        ("[FAIL]", "real_src/django_settings.py", "django", "not declared"),
        ("[FAIL]", "real_src/module3/content.py", "django", "not declared"),
    ]

    _check_expected_messages_unordered(general_section, expected_general)
    _check_expected_messages_unordered(external_section, expected_external)
