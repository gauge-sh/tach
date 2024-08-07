from __future__ import annotations

from typing import TYPE_CHECKING

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


def find_modules_with_cycles(
    modules: list[ModuleConfig],
) -> list[str]:
    # Local import because networkx takes about ~100ms to load
    import networkx as nx
    from networkx import NetworkXNoCycle

    graph = nx.DiGraph()  # type: ignore
    # Add nodes
    for module in modules:
        graph.add_node(module.path)  # type: ignore

    # Add dependency edges
    for module in modules:
        for dependency in module.depends_on:
            graph.add_edge(module.path, dependency.path)  # type: ignore

    modules_with_cycles: list[str] = []
    for module in modules:
        module_path = module.path
        try:
            # Find *any* cycle, starting with module_path
            cycle: list[tuple[str, str]] = nx.find_cycle(graph, source=module_path)  # type: ignore
            for edge in cycle:  # type: ignore
                # Confirm that the cycle includes module_path
                if module_path in edge:
                    modules_with_cycles.append(module.path)
                    break
        except NetworkXNoCycle:
            return []

    return modules_with_cycles


def build_module_tree(
    source_roots: list[Path],
    modules: list[ModuleConfig],
    forbid_circular_dependencies: bool,
) -> ModuleTree:
    duplicate_modules = find_duplicate_modules(modules)
    if duplicate_modules:
        raise ValueError(
            f"Failed to build module tree. The following modules were defined more than once: {duplicate_modules}"
        )
    if forbid_circular_dependencies:
        module_paths = find_modules_with_cycles(modules)
        if module_paths:
            raise TachCircularDependencyError(module_paths)
    tree = ModuleTree()
    for module in modules:
        tree.insert(
            config=module,
            path=module.mod_path,
            interface_members=parse_interface_members(
                source_roots=source_roots, module_path=module.path
            ),
        )
    return tree
