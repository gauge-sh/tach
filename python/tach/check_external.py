from __future__ import annotations

from dataclasses import dataclass, field
from typing import TYPE_CHECKING

# from tach.extension import check_external_dependencies

if TYPE_CHECKING:
    from pathlib import Path

    from tach.core.config import ProjectConfig


@dataclass
class ExternalCheckResult:
    errors: list[str] = field(default_factory=list)
    warnings: list[str] = field(default_factory=list)


def check_external(
    project_root: Path, project_config: ProjectConfig
) -> ExternalCheckResult:
    # project config not used at the moment
    # result = check_external_dependencies(
    #     project_root=str(project_root),
    # )
    # return ExternalCheckResult(
    #     errors=result[0],
    #     warnings=result[1],
    # )
    return ExternalCheckResult()
