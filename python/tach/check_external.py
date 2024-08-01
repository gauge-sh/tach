from __future__ import annotations

from dataclasses import dataclass, field
from typing import TYPE_CHECKING

from tach.extension import check_external_dependencies

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
    serialized_source_roots = [
        str(project_root / source_root) for source_root in project_config.source_roots
    ]
    result = check_external_dependencies(
        project_root=str(project_root),
        source_roots=serialized_source_roots,
        ignore_type_checking_imports=project_config.ignore_type_checking_imports,
    )
    return ExternalCheckResult(
        errors=result[0],
        warnings=result[1],
    )
