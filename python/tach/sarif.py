from __future__ import annotations

import json
from pathlib import Path
from typing import TYPE_CHECKING, Any

from tach import __version__
from tach.utils.display import build_absolute_error_path, build_error_message

if TYPE_CHECKING:
    from tach.check import BoundaryError


def create_results() -> dict[str, str | list[Any] | dict[str, Any]]:
    return {
        "version": "2.1.0",
        "$schema": "https://docs.oasis-open.org/sarif/sarif/v2.1.0/errata01/os/schemas/sarif-schema-2.1.0.json",
        "runs": [
            {
                "tool": {
                    "driver": {
                        "name": "Tach",
                        "informationUri": "https://github.com/gauge-sh/tach",
                        "version": __version__,
                    }
                },
                "results": [],
            }
        ],
    }


def build_sarif_errors(
    errors: list[BoundaryError], source_roots: list[Path], project_root: Path
) -> list[dict[str, Any]]:
    sarif_errors: list[dict[str, Any]] = []
    for error in errors:
        absolute_path = build_absolute_error_path(
            file_path=error.file_path, source_roots=source_roots
        )
        relative_path = absolute_path.relative_to(project_root)
        sarif_errors.append(
            {
                "level": "warning" if error.error_info.is_deprecated else "error",
                "ruleId": "tach",
                "message": {
                    "text": build_error_message(error=error, source_roots=source_roots)
                },
                "locations": [
                    {
                        "physicalLocation": {
                            "artifactLocation": {
                                "uri": str(relative_path),
                            },
                            "region": {
                                "startLine": 1,
                                "startColumn": error.line_number,
                            },
                        }
                    }
                ],
            }
        )
    return sarif_errors


def write_sarif_file(
    sarif_results: dict[str, str | list[Any] | dict[str, Any]],
) -> None:
    with open(Path.cwd() / "tach-check-results.sarif", "w") as f:
        f.write(json.dumps(sarif_results, indent=2))


__all__ = [
    "build_sarif_errors",
    "create_results",
    "write_sarif_file",
]
