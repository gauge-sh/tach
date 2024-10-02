from __future__ import annotations

import json
from copy import copy
from json.decoder import JSONDecodeError
from typing import TYPE_CHECKING
from urllib import error, request

from tach import filesystem as fs
from tach.extension import ModuleConfig, ProjectConfig

if TYPE_CHECKING:
    from pathlib import Path

    import pydot  # type: ignore


TACH_SHOW_URL = "https://show.gauge.sh"


def generate_show_url(
    project_root: Path,
    project_config: ProjectConfig,
    included_paths: list[Path] | None = None,
) -> str | None:
    if included_paths:
        project_config = filter_project_config(
            project_config, project_root=project_root, included_paths=included_paths
        )
    json_data = project_config.model_dump_json()
    json_bytes = json_data.encode("utf-8")
    req = request.Request(
        f"{TACH_SHOW_URL}/api/core/0.6.5/graph/",
        data=json_bytes,
        headers={"Content-Type": "application/json"},
    )

    try:
        # Send the request and read the response
        with request.urlopen(req) as response:
            response_data = response.read().decode("utf-8")
            response_json = json.loads(response_data)
            url = response_json.get("uid")
            return f"{TACH_SHOW_URL}?uid={url}"
    except (UnicodeDecodeError, JSONDecodeError, error.URLError) as e:
        print(f"Error: {e}")
        return None


def module_path_is_included_in_paths(
    source_roots: tuple[Path, ...], module_path: str, included_paths: list[Path] | None
) -> bool:
    if included_paths is None:
        return True
    module_fs_path = fs.module_to_pyfile_or_dir_path(source_roots, module_path)
    if not module_fs_path:
        return False
    for included_path in included_paths:
        if included_path in module_fs_path.parents or included_path == module_fs_path:
            return True
    return False


def filter_project_config(
    project_config: ProjectConfig,
    project_root: Path,
    included_paths: list[Path],
) -> ProjectConfig:
    source_roots = tuple(
        map(lambda source_root: project_root / source_root, project_config.source_roots)
    )
    included_paths = list(map(lambda path: project_root / path, included_paths))
    all_modules = copy(project_config.modules)
    project_config.set_modules([])
    filtered_modules: list[ModuleConfig] = []
    for module in all_modules:
        if module_path_is_included_in_paths(source_roots, module.path, included_paths):
            filtered_modules.append(module)
    return ProjectConfig.with_modules(project_config, filtered_modules)


def generate_module_graph_dot_file(
    project_root: Path,
    project_config: ProjectConfig,
    output_filepath: Path,
    included_paths: list[Path] | None = None,
) -> None:
    # Local import because networkx takes about ~100ms to load
    import networkx as nx

    graph = nx.DiGraph()  # type: ignore

    def upsert_edge(graph: nx.DiGraph, module: str, dependency: str) -> None:  # type: ignore
        if module not in graph:
            graph.add_node(module)  # type: ignore
        if dependency not in graph:
            graph.add_node(dependency)  # type: ignore
        graph.add_edge(module, dependency)  # type: ignore

    source_roots = tuple(
        map(lambda source_root: project_root / source_root, project_config.source_roots)
    )
    included_paths = (
        list(map(lambda path: project_root / path, included_paths))
        if included_paths
        else None
    )
    for module in project_config.modules:
        module_is_included = module_path_is_included_in_paths(
            source_roots, module.path, included_paths
        )
        for dependency in module.depends_on:
            dependency_is_included = module_path_is_included_in_paths(
                source_roots, dependency.path, included_paths
            )

            # This essentially means we propagate one degree from the included modules
            if module_is_included or dependency_is_included:
                upsert_edge(graph, module.path, dependency.path)  # type: ignore

    pydot_graph: pydot.Dot = nx.nx_pydot.to_pydot(graph)  # type: ignore
    dot_data: str = pydot_graph.to_string()  # type: ignore

    output_filepath.write_text(dot_data)  # type: ignore


def generate_module_graph_mermaid(
    project_root: Path,
    project_config: ProjectConfig,
    output_filepath: Path,
    included_paths: list[Path] | None = None,
):
    if included_paths:
        project_config = filter_project_config(
            project_config, project_root, included_paths
        )
    edges: list[str] = []
    isolated: list[str] = []
    for module in project_config.modules:
        for dependency in module.depends_on:
            edges.append(
                f"    {module.path.strip('<>')} --> {dependency.path.strip('<>')}"
            )
        if not module.depends_on:
            isolated.append(f"    {module.path.strip('<>')}")

    mermaid_graph = "graph TD\n" + "\n".join(edges) + "\n" + "\n".join(isolated)

    output_filepath.write_text(mermaid_graph)


__all__ = [
    "generate_show_url",
    "generate_module_graph_dot_file",
    "generate_module_graph_mermaid",
]
