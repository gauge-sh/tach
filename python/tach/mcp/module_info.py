from __future__ import annotations

import xml.etree.ElementTree as ET
from pathlib import Path
from xml.dom import minidom

from tach.parsing import parse_project_config


def get_module_info_xml(module_path: str) -> str:
    """
    Generate an XML representation of a specific module's information.
    
    Args:
        module_path: Path to the module (e.g., 'tach.parsing')
        
    Returns:
        XML string representing the module information
    """
    # We need to find the project root to parse the project config
    # For now, we'll use the current directory as the project root
    project_root = Path.cwd()
    config = parse_project_config(project_root)
    
    if not config:
        return "Error: Could not parse project config. The current directory may not contain a valid Tach project."
    
    # Find the module in the project config
    module_config = None
    for mod in config.all_modules():
        if mod.path == module_path:
            module_config = mod
            break
    
    if not module_config:
        return f"Error: Module '{module_path}' not found in the project config."
    
    # Create the root element
    root = ET.Element("module")
    root.set("path", module_config.path)
    
    # Add module attributes
    if module_config.layer:
        root.set("layer", module_config.layer)
        
    if module_config.unchecked:
        root.set("unchecked", str(module_config.unchecked).lower())
    
    # Add dependencies
    if module_config.depends_on:
        dependencies_element = ET.SubElement(root, "dependencies")
        for dependency in module_config.depends_on:
            dependency_element = ET.SubElement(dependencies_element, "dependency")
            dependency_element.set("path", dependency.path)
            if dependency.deprecated:
                dependency_element.set("deprecated", str(dependency.deprecated).lower())
    
    # Add visibility
    if module_config.visibility:
        visibility_element = ET.SubElement(root, "visibility")
        for visible_to in module_config.visibility:
            visible_element = ET.SubElement(visibility_element, "visible-to")
            visible_element.text = visible_to
    
    # Add dependents (modules that depend on this module)
    dependents_element = ET.SubElement(root, "dependents")
    for mod in config.all_modules():
        if mod.depends_on:
            for dep in mod.depends_on:
                if dep.path == module_path:
                    dependent_element = ET.SubElement(dependents_element, "dependent")
                    dependent_element.set("path", mod.path)
                    if mod.layer:
                        dependent_element.set("layer", mod.layer)
                    break
    
    # Add interface information
    interface_element = ET.SubElement(root, "interface")
    for interface in config.all_interfaces():
        if module_path in interface.from_modules:
            for member in interface.expose:
                member_element = ET.SubElement(interface_element, "member")
                member_element.text = member
            interface_element.set("data_types", interface.data_types)
            break
    
    # Convert to string with pretty formatting
    rough_string = ET.tostring(root, 'utf-8')
    reparsed = minidom.parseString(rough_string)
    return reparsed.toprettyxml(indent="  ")
