from __future__ import annotations

import json
from pathlib import Path
from typing import TYPE_CHECKING, Any

from tach.utils.display import build_absolute_error_path, build_error_message

if TYPE_CHECKING:
    from tach.check import BoundaryError


def create_results() -> dict[str, str | list[Any] | dict[str, Any]]:
    return {  # pyright: ignore [reportUnknownVariableType]
        "version": "2.1.0",
        "$schema": "http://json.schemastore.org/sarif-2.1.0-rtm.4",
        "runs": [
            {
                "tool": {
                    "driver": {
                        "name": "Tach",
                        "informationUri": "https://github.com/gauge-sh/tach",
                        # "rules": [
                        # {
                        #   "id": "no-unused-vars",
                        #   "shortDescription": {
                        #     "text": "disallow unused variables"
                        #   },
                        #   "helpUri": "https://eslint.org/docs/rules/no-unused-vars",
                        #   "properties": {
                        #     "category": "Variables"
                        #   }
                        # }
                        # ],
                    }
                },
                # "artifacts": [
                #     {
                #         "location": {
                #             "uri": "file:///C:/dev/sarif/sarif-tutorials/samples/Introduction/simple-example.js"
                #         }
                #     }
                # ],
                "results": [
                    # {
                    #     "level": "error",
                    #     "message": {"text": "'x' is assigned a value but never used."},
                    #     "locations": [
                    #         {
                    #             "physicalLocation": {
                    #                 "artifactLocation": {
                    #                     "uri": "file:///C:/dev/sarif/sarif-tutorials/samples/Introduction/simple-example.js",
                    #                     "index": 0,
                    #                 },
                    #                 "region": {"startLine": 1, "startColumn": 5},
                    #             }
                    #         }
                    #     ],
                    #     "ruleId": "no-unused-vars",
                    #     "ruleIndex": 0,
                    # }
                ],
            }
        ],
    }


def build_sarif_errors(
    errors: list[BoundaryError], source_roots: list[Path]
) -> list[dict[str, Any]]:
    return [
        {
            "level": "warning" if error.error_info.is_deprecated else "error",
            "message": build_error_message(error=error, source_roots=source_roots),
            "locations": [
                {
                    "physicalLocation": {
                        "artifactLocation": {
                            "uri": build_absolute_error_path(
                                file_path=error.file_path, source_roots=source_roots
                            ),
                            "index": 0,
                        },
                        "region": {"startLine": 1, "startColumn": error.line_number},
                    }
                }
            ],
        }
        for error in errors
    ]


def write_sarif_file(
    sarif_results: dict[str, str | list[Any] | dict[str, Any]],
) -> None:
    with open(Path.cwd() / "tach-check-results.sarif", "w") as f:
        f.write(json.dumps(sarif_results))


__all__ = [
    "build_sarif_errors",
    "create_results",
]
