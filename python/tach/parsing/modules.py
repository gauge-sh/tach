from __future__ import annotations

from typing import TYPE_CHECKING

import networkx as nx

from tach.core import ModuleTree
from tach.errors import TachCircularDependencyError
from tach.parsing import parse_interface_members

if TYPE_CHECKING:
    from pathlib import Path

    from tach.core import ModuleConfig


def find_duplicate_modules(modules: list[ModuleConfig]) -> list[str]:
    duplicate_module_paths: list[str] = []
    seen: set[str] = set()
    for module in modules:
        if module.path in seen:
            duplicate_module_paths.append(module.path)
            continue
        seen.add(module.path)
    return duplicate_module_paths


def canonical_form(cycle: list[str]) -> list[str]:
    """Rotate cycle to start with the smallest element."""
    min_index = cycle.index(min(cycle))
    return cycle[min_index:] + cycle[:min_index]


def find_cycles(
    modules: list[ModuleConfig],
) -> list[list[str]]:
    graph = nx.DiGraph()  # type: ignore
    # Add nodes
    for module in modules:
        graph.add_node(module.path)  # type: ignore

    # Add dependency edges
    for module in modules:
        for dependency in module.depends_on:
            graph.add_edge(module.path, dependency.path)  # type: ignore

    all_cycles: list[list[str]] = list(nx.simple_cycles(graph))  # type: ignore

    canonical_cycles = {tuple(canonical_form(cycle)) for cycle in all_cycles}

    unique_cycles = [list(cycle) for cycle in canonical_cycles]

    return unique_cycles


def build_module_tree(
    source_root: Path, modules: list[ModuleConfig], forbid_circular_dependencies: bool
) -> ModuleTree:
    duplicate_modules = find_duplicate_modules(modules)
    if duplicate_modules:
        raise ValueError(
            f"Failed to build module tree. The following modules were defined more than once: {duplicate_modules}"
        )
    if forbid_circular_dependencies:
        cycles = find_cycles(modules)
        if cycles:
            raise TachCircularDependencyError(cycles)
    tree = ModuleTree()
    for module in modules:
        tree.insert(
            config=module,
            path=module.mod_path,
            interface_members=parse_interface_members(
                source_root=source_root, module_path=module.path
            ),
        )
    return tree
