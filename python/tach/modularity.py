from __future__ import annotations

import json
import os
from dataclasses import asdict, dataclass, field
from http.client import HTTPConnection, HTTPSConnection
from pathlib import Path
from typing import Any
from urllib import parse

from tach import filesystem as fs
from tach.errors import TachError
from tach.extension import (
    CheckDiagnostics,
    InterfaceRuleConfig,
    ProjectConfig,
    check,
    get_project_imports,
    parse_interface_members,
)
from tach.filesystem.git_ops import get_current_branch_info
from tach.parsing.config import extend_and_validate


def export_modularity(
    project_root: Path,
    project_config: ProjectConfig,
    upload: bool = False,
    output_path: Path | None = None,
    force: bool = False,
):
    report = generate_modularity_report(project_root, project_config, force=force)

    if upload:
        upload_report(report, project_config)
    else:
        output_path = output_path or project_root / "modularity_report.json"
        output_path.write_text(json.dumps(asdict(report), indent=2))


GAUGE_API_KEY = os.getenv("GAUGE_API_KEY", "")
GAUGE_API_BASE_URL = os.getenv("GAUGE_API_BASE_URL", "http://localhost:8000")


def build_modularity_upload_path(repo: str) -> str:
    return f"/api/client/repos/{repo}/modularity"


def post_json_to_gauge_api(path: str, data: dict[str, Any]) -> None:
    if not GAUGE_API_KEY:
        raise TachError("GAUGE_API_KEY is not set.")
    headers = {
        "Content-Type": "application/json",
        "Authorization": GAUGE_API_KEY,
    }
    json_data = json.dumps(data)
    conn = None
    full_url = f"{GAUGE_API_BASE_URL}{path}"
    try:
        url_parts: parse.ParseResult = parse.urlparse(full_url)
        if full_url.startswith("https://"):
            conn = HTTPSConnection(url_parts.netloc, timeout=10)
        else:
            conn = HTTPConnection(url_parts.netloc, timeout=10)
        conn.request("POST", path, body=json_data, headers=headers)
        response = conn.getresponse()

        # Check for non-200 status codes
        if response.status != 200:
            error_message = response.read().decode("utf-8")
            raise TachError(
                f"API request failed with status {response.status}: {error_message}"
            )

    except Exception as e:
        raise TachError(f"Failed to upload modularity report: {str(e)}")
    finally:
        if conn is not None:
            conn.close()


def upload_report(report: Report, _project_config: ProjectConfig):
    print("Uploading report...")
    path = build_modularity_upload_path(report.repo)
    post_json_to_gauge_api(path, asdict(report))
    print("Report uploaded!")


# NOTE: these usages are all imports
@dataclass
class Usage:
    # The configured Module being used
    module_path: str
    # The full import path to the member being used
    full_path: str
    # The file containing the usage
    filepath: str
    # 1-indexed location of the usage in the file
    line: int
    # [1.1] The Module that contains the Usage (None indicates Usage outside of any Module)
    containing_module_path: str | None


@dataclass
class Module:
    path: str
    is_strict: bool = False
    interface_members: list[str] = field(default_factory=list)


@dataclass
class InterfaceRule:
    matches: list[str]
    for_modules: list[str] = field(default_factory=lambda: ["*"])


REPORT_VERSION = "1.1"


@dataclass
class ReportMetadata:
    version: str = REPORT_VERSION
    configuration_format: str = "json"


@dataclass
class ErrorInfo:
    is_deprecated: bool
    pystring: str


@dataclass
class BoundaryError:
    file_path: Path
    line_number: int
    import_mod_path: str
    error_info: ErrorInfo


@dataclass
class CheckResult:
    errors: list[BoundaryError] = field(default_factory=list)
    deprecated_warnings: list[BoundaryError] = field(default_factory=list)
    warnings: list[str] = field(default_factory=list)


@dataclass
class Report:
    repo: str
    branch: str
    commit: str
    # [1.1] The full configuration encoded as JSON
    full_configuration: str
    modules: list[Module] = field(default_factory=list)
    usages: list[Usage] = field(default_factory=list)
    interface_rules: list[InterfaceRule] = field(default_factory=list)
    check_result: CheckResult = field(default_factory=CheckResult)
    metadata: ReportMetadata = field(default_factory=ReportMetadata)


def build_modules(
    source_roots: list[Path], project_config: ProjectConfig
) -> list[Module]:
    modules: list[Module] = []
    for module in project_config.modules:
        if module.mod_path() == ".":
            # Skip <root>
            continue

        if module.strict:
            # If module is strict, parse and add interface members
            interface_members = parse_interface_members(
                source_roots,
                module.mod_path(),
            )

            modules.append(
                Module(
                    path=module.path,
                    is_strict=True,
                    interface_members=interface_members,
                )
            )
        else:
            modules.append(Module(path=module.path))

    return modules


def build_usages(
    project_root: Path, source_roots: list[Path], project_config: ProjectConfig
) -> list[Usage]:
    module_paths = sorted(
        (module.path for module in project_config.modules),
        key=lambda path: len(path.split(".")),
        reverse=True,
    )

    def get_containing_module(mod_path: str) -> str | None:
        return next(
            (
                module_path
                for module_path in module_paths
                if mod_path == module_path or mod_path.startswith(f"{module_path}.")
            ),
            None,
        )

    usages: list[Usage] = []
    for pyfile in fs.walk_pyfiles(
        project_root,
        project_root=project_root,
        exclude_paths=project_config.exclude,
        use_regex_matching=project_config.use_regex_matching,
    ):
        pyfile_mod_path = fs.file_to_module_path(
            tuple(source_roots), project_root / pyfile
        )
        pyfile_containing_module = get_containing_module(pyfile_mod_path)
        imports = get_project_imports(
            source_roots=list(map(str, source_roots)),
            file_path=str(pyfile),
            ignore_type_checking_imports=project_config.ignore_type_checking_imports,
            include_string_imports=project_config.include_string_imports,
        )
        for project_import in imports:
            import_mod_path, line = project_import
            import_containing_module = get_containing_module(import_mod_path)
            if (
                import_containing_module is None
                or import_containing_module == pyfile_containing_module
            ):
                continue

            usages.append(
                Usage(
                    module_path=import_containing_module,
                    full_path=import_mod_path,
                    filepath=str(pyfile),
                    line=line,
                    containing_module_path=pyfile_containing_module,
                )
            )

    return usages


def build_interface_rules(
    interface_rules: list[InterfaceRuleConfig],
) -> list[InterfaceRule]:
    # TODO: validate interface rules
    return [
        InterfaceRule(matches=rule.matches, for_modules=rule.for_modules)
        for rule in interface_rules
    ]


def process_check_result(check_diagnostics: CheckDiagnostics) -> CheckResult:
    return CheckResult(
        errors=[
            BoundaryError(
                file_path=error.file_path,
                line_number=error.line_number,
                import_mod_path=error.import_mod_path,
                error_info=ErrorInfo(
                    is_deprecated=error.error_info.is_deprecated(),
                    pystring=error.error_info.to_pystring(),
                ),
            )
            for error in check_diagnostics.errors
        ],
        deprecated_warnings=[
            BoundaryError(
                file_path=Path(warning.file_path),
                line_number=warning.line_number,
                import_mod_path=warning.import_mod_path,
                error_info=ErrorInfo(
                    is_deprecated=warning.error_info.is_deprecated(),
                    pystring=warning.error_info.to_pystring(),
                ),
            )
            for warning in check_diagnostics.deprecated_warnings
        ],
        warnings=check_diagnostics.warnings,
    )


def generate_modularity_report(
    project_root: Path, project_config: ProjectConfig, force: bool = False
) -> Report:
    print("Generating report...")
    branch_info = get_current_branch_info(project_root, allow_dirty=force)
    report = Report(
        repo=branch_info.repo,
        branch=branch_info.name,
        commit=branch_info.commit,
        full_configuration=project_config.model_dump_json(),
    )
    source_roots = [project_root / root for root in project_config.source_roots]
    exclude_paths = extend_and_validate(
        None, project_config.exclude, project_config.use_regex_matching
    )
    check_diagnostics = check(
        project_root=project_root,
        project_config=project_config,
        exclude_paths=exclude_paths,
    )
    report.modules = build_modules(source_roots, project_config)
    report.usages = build_usages(project_root, source_roots, project_config)
    report.interface_rules = build_interface_rules(
        project_config.gauge.valid_interface_rules
    )
    report.check_result = process_check_result(check_diagnostics)

    print("Report generated!")
    return report


__all__ = ["export_modularity"]
