from __future__ import annotations

import json
import os
import re
from dataclasses import asdict, dataclass, field
from http.client import HTTPConnection, HTTPSConnection
from typing import TYPE_CHECKING, Any
from urllib import parse

from tach import filesystem as fs
from tach.colors import BCOLORS
from tach.errors import TachClosedBetaError, TachError
from tach.extension import (
    ProjectConfig,
    get_project_imports,
)
from tach.filesystem.git_ops import get_current_branch_info

if TYPE_CHECKING:
    from pathlib import Path


def export_report(
    project_root: Path,
    project_config: ProjectConfig,
    output_path: Path | None = None,
    force: bool = False,
):
    """Export a modularity report to a local file."""
    report = generate_modularity_report(project_root, project_config, force=force)
    output_path = output_path or project_root / "modularity_report.json"
    output_path.write_text(json.dumps(asdict(report), indent=2))


def upload_report_to_gauge(
    project_root: Path,
    project_config: ProjectConfig,
    force: bool = False,
):
    """Upload a modularity report to Gauge."""
    report = generate_modularity_report(project_root, project_config, force=force)
    print(f"{BCOLORS.OKCYAN} > Uploading report...{BCOLORS.ENDC}")
    path = build_modularity_upload_path(report.repo)
    post_json_to_gauge_api(path, asdict(report))
    print(f"{BCOLORS.OKGREEN} > Report uploaded!{BCOLORS.ENDC}")


GAUGE_API_KEY = os.getenv("GAUGE_API_KEY", "")
GAUGE_API_BASE_URL = os.getenv("GAUGE_API_BASE_URL", "https://app.gauge.sh")


def build_modularity_upload_path(repo: str) -> str:
    return f"/api/client/repos/{repo}/modularity"


def post_json_to_gauge_api(path: str, data: dict[str, Any]) -> None:
    if not GAUGE_API_KEY:
        raise TachClosedBetaError(
            f"{BCOLORS.WARNING}Modularity is currently in closed beta. Visit {GAUGE_API_BASE_URL}/closed-beta to request access.{BCOLORS.ENDC}"
            "\n\n"
            f"{BCOLORS.OKCYAN}Already have access? Set the GAUGE_API_KEY environment variable to continue.{BCOLORS.ENDC}"
        )
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
    # [1.2] Deprecated
    is_strict: bool = False
    # [1.2] Replaces 'is_strict'
    has_interface: bool = False
    interface_members: list[str] = field(default_factory=list)


REPORT_VERSION = "1.2"


@dataclass
class ReportMetadata:
    version: str = REPORT_VERSION
    configuration_format: str = "json"


@dataclass
class Report:
    repo: str
    branch: str
    commit: str
    # [1.1] The full configuration encoded as JSON
    full_configuration: str
    modules: list[Module] = field(default_factory=list)
    usages: list[Usage] = field(default_factory=list)
    # [1.2] Deprecated
    interface_rules: list[Any] = field(default_factory=list)
    metadata: ReportMetadata = field(default_factory=ReportMetadata)


def build_modules(project_config: ProjectConfig) -> list[Module]:
    modules: list[Module] = []
    for module in project_config.modules:
        if module.mod_path() == ".":
            # Skip <root>
            continue
        has_interface = False
        interface_members: set[str] = set()
        for interface in project_config.interfaces:
            if any(
                re.match(pattern, module.path) for pattern in interface.from_modules
            ):
                has_interface = True
                interface_members.update(interface.expose)

        modules.append(
            Module(
                path=module.path,
                has_interface=has_interface,
                interface_members=list(interface_members),
            )
        )

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


def generate_modularity_report(
    project_root: Path, project_config: ProjectConfig, force: bool = False
) -> Report:
    print(f"{BCOLORS.OKCYAN} > Generating report...{BCOLORS.ENDC}")
    branch_info = get_current_branch_info(project_root, allow_dirty=force)
    report = Report(
        repo=branch_info.repo,
        branch=branch_info.name,
        commit=branch_info.commit,
        full_configuration=project_config.model_dump_json(),
    )
    source_roots = [project_root / root for root in project_config.source_roots]

    report.modules = build_modules(project_config)
    report.usages = build_usages(project_root, source_roots, project_config)

    print(f"{BCOLORS.OKGREEN} > Report generated!{BCOLORS.ENDC}")
    return report


__all__ = ["export_report", "upload_report_to_gauge"]
