from __future__ import annotations

from pathlib import Path

from mcp.server.fastmcp import FastMCP

from tach.constants import TOOL_NAME
from tach.mcp.dependency_report import dependency_report
from tach.mcp.module_info import get_module_info_xml
from tach.mcp.project_structure import get_project_structure_xml

mcp = FastMCP(TOOL_NAME)

@mcp.tool()  # type: ignore
def get_project_structure(project_root: Path) -> str:
    """
    Get a list of all the tracked Python modules in the project.
    
    Each module may have a set of configured dependencies, which are other modules that it can import from.
    Each module may also belong to a 'layer', which determines its place in the module hierarchy and also influences which modules it can import from.

    Use this tool to get a high-level overview of the project structure. This is most useful to plan changes across multiple files.
    
    Args:
        project_root: Path to the root directory of the project.
    """
    return get_project_structure_xml(project_root)

@mcp.tool()  # type: ignore
def get_module_info(module_path: str) -> str:
    """
    Get detailed information about a specific module.

    This includes the module's dependencies, dependents, public interface, layer, and more.

    Use this tool to get a detailed overview of a specific module when planning where to make changes.
    
    Args:
        module_path: Dot-separated path to the module to get information about. (ex: 'mypkg.mymodule')
    """
    return get_module_info_xml(module_path)

@mcp.tool()  # type: ignore
def get_dependency_report(project_root: Path, file_path_or_directory: Path) -> str:
    """
    Get a report on the dependencies of a specific file or directory.

    This includes actual source code locations of dependencies on other modules,
    dependencies on 3rd party packages, and usages of the file or directory elsewhere in the codebase.
    
    Use this tool to get a detailed overview of a specific file or directory when planning where to make changes.
    
    Args:
        project_root: Path to the root directory of the project.
        file_path_or_directory: Path to the file or directory to analyze.
    """
    return dependency_report(project_root, file_path_or_directory)


if __name__ == "__main__":
    mcp.run(transport="stdio")
