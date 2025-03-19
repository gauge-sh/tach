from __future__ import annotations

import json

from tach.modularity import export_report
from tach.parsing.config import parse_project_config


def test_export_report_diagnostics(example_dir, tmp_path):
    example_project = example_dir / "many_features"

    project_config = parse_project_config(root=example_project)
    assert project_config is not None

    output_path = tmp_path / "modularity_report.json"

    export_report(example_project, project_config, output_path, force=True)

    assert output_path.exists()

    with open(output_path) as f:
        report = json.load(f)

    assert "diagnostics" in report
    assert isinstance(report["diagnostics"], str)

    diagnostics = json.loads(report["diagnostics"])

    assert isinstance(diagnostics, list)
    assert len(diagnostics) > 0

    for diagnostic in diagnostics:
        assert len(diagnostic) == 1
        variant = next(iter(diagnostic))
        assert variant in ("Located", "Global")

        if variant == "Located":
            located = diagnostic[variant]
            assert "file_path" in located
            assert "line_number" in located
            assert "severity" in located
            assert "details" in located
            assert located["severity"] in ("Error", "Warning")

            details = located["details"]
            assert len(details) == 1
            detail_type = next(iter(details))
            assert detail_type in ("Code", "Configuration")

        elif variant == "Global":
            global_diag = diagnostic[variant]
            assert "severity" in global_diag
            assert "details" in global_diag
            assert global_diag["severity"] in ("Error", "Warning")

            details = global_diag["details"]
            assert len(details) == 1
            detail_type = next(iter(details))
            assert detail_type in ("Code", "Configuration")
