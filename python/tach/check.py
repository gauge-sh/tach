from __future__ import annotations

from typing import TYPE_CHECKING

from tach.extension import Diagnostic
from tach.extension import check as ext_check

if TYPE_CHECKING:
    from pathlib import Path

    from tach.extension import ProjectConfig


def check(
    project_root: Path,
    project_config: ProjectConfig,
    dependencies: bool = True,
    interfaces: bool = True,
) -> list[Diagnostic]:
    return ext_check(
        project_root, project_config, dependencies=dependencies, interfaces=interfaces
    )
