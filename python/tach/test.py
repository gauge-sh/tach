from __future__ import annotations

from copy import copy
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any

from tach.errors import TachSetupError
from tach.extension import ProjectConfig, TachPytestPluginHandler
from tach.filesystem.git_ops import get_changed_files

if TYPE_CHECKING:
    from pathlib import Path


@dataclass
class AffectedTestsResult:
    exit_code: int
    tests_ran_to_completion: bool


def run_affected_tests(
    project_root: Path,
    project_config: ProjectConfig,
    head: str,
    base: str,
    pytest_args: list[Any] | None = None,
) -> AffectedTestsResult:
    try:
        import pytest  # type: ignore  # noqa: F401
    except ImportError:
        raise TachSetupError("Cannot run tests, could not find 'pytest'.")

    class TachPytestPlugin:
        def __init__(
            self,
            handler: TachPytestPluginHandler,
        ):
            self.handler = handler

        def pytest_collection_modifyitems(
            self,
            session: pytest.Session,
            config: pytest.Config,
            items: list[pytest.Item],
        ):
            seen: set[Path] = set()
            for item in copy(items):
                if not item.path:
                    continue
                if item.path in self.handler.removed_test_paths:
                    self.handler.num_removed_items += 1
                    items.remove(item)
                    continue
                if item.path in seen:
                    continue

                if item.path in self.handler.all_affected_modules:
                    # If this test file was changed,
                    # then we know we need to rerun it
                    seen.add(item.path)
                    continue

                if self.handler.should_remove_items(file_path=item.path.resolve()):
                    self.handler.num_removed_items += 1
                    items.remove(item)
                    self.handler.removed_test_paths.add(item.path)

                seen.add(item.path)

        def pytest_report_collectionfinish(
            self,
            config: pytest.Config,
            start_path: Path,
            startdir: Any,
            items: list[pytest.Item],
        ) -> str | list[str]:
            return [
                f"[Tach] Skipped {len(self.handler.removed_test_paths)} test file{'s' if len(self.handler.removed_test_paths) > 1 else ''}"
                f" ({self.handler.num_removed_items} tests)"
                " since they were unaffected by current changes.",
                *(
                    f"[Tach] > Skipped '{test_path}'"
                    for test_path in self.handler.removed_test_paths
                ),
            ]

        def pytest_terminal_summary(
            self, terminalreporter: Any, exitstatus: int, config: pytest.Config
        ):
            self.handler.tests_ran_to_completion = True

    changed_files = get_changed_files(project_root, head=head, base=base)
    pytest_plugin_handler = TachPytestPluginHandler(
        project_root=project_root,
        project_config=project_config,
        changed_files=changed_files,
        all_affected_modules={changed_file.resolve() for changed_file in changed_files},
    )

    exit_code = pytest.main(
        pytest_args, plugins=[TachPytestPlugin(handler=pytest_plugin_handler)]
    )

    if exit_code == pytest.ExitCode.NO_TESTS_COLLECTED:
        # Selective testing means running zero tests will happen regularly,
        # so we do not want the default behavior of failing when no tests
        # are collected.
        exit_code = pytest.ExitCode.OK

    return AffectedTestsResult(
        exit_code=exit_code,
        tests_ran_to_completion=pytest_plugin_handler.tests_ran_to_completion,
    )


__all__ = ["run_affected_tests"]
