from __future__ import annotations

import json
from json.decoder import JSONDecodeError
from typing import TYPE_CHECKING
from urllib import error, request

if TYPE_CHECKING:
    from pathlib import Path

    import pydot  # type: ignore

    from tach.extension import ProjectConfig

TACH_SHOW_URL = "https://show.gauge.sh"


def generate_show_url(project_config: ProjectConfig) -> str | None:
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


def generate_module_graph_dot_file(
    project_config: ProjectConfig, output_filepath: Path
) -> None:
    # Local import because networkx takes about ~100ms to load
    import networkx as nx

    graph = nx.DiGraph()  # type: ignore
    # Add nodes
    for module in project_config.modules:
        graph.add_node(module.path)  # type: ignore

    # Add dependency edges
    for module in project_config.modules:
        for dependency in module.depends_on:
            graph.add_edge(module.path, dependency.path)  # type: ignore

    pydot_graph: pydot.Dot = nx.nx_pydot.to_pydot(graph)  # type: ignore
    dot_data: str = pydot_graph.to_string()  # type: ignore

    output_filepath.write_text(dot_data)  # type: ignore


__all__ = ["generate_show_url", "generate_module_graph_dot_file"]
