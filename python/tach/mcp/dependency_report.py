from __future__ import annotations

from pathlib import Path

from tach.parsing import parse_project_config
from tach.report import external_dependency_report, report


def dependency_report(project_root: Path, file_path_or_directory: Path) -> str:
    """
    Generate a comprehensive dependency report for a file or directory.
    
    This combines both the standard dependency report and the external dependency report.
    
    Args:
        project_root: Path to the project root
        file_path_or_directory: Path to the file or directory to analyze
        
    Returns:
        A string containing the combined dependency reports
    """
    project_config = parse_project_config(project_root)
    if project_config is None:
        return f"Error: Could not parse project config. The path '{project_root}' may not contain a valid Tach project."
    
    standard_report = report(
        project_root=project_root,
        path=file_path_or_directory,
        project_config=project_config,
        raw=False
    )
    
    external_report = external_dependency_report(
        project_root=project_root,
        path=file_path_or_directory,
        project_config=project_config,
        raw=False
    )
    
    combined_report = f"{standard_report}\n\n{'=' * 80}\n\nEXTERNAL DEPENDENCIES\n\n{external_report}"
    
    return combined_report
