from __future__ import annotations

import xml.etree.ElementTree as ET
from pathlib import Path
from xml.dom import minidom

from tach.parsing import parse_project_config


def get_project_structure_xml(project_root: Path) -> str:
    """
    Generate an XML representation of the project structure.
    
    Args:
        project_root: Path to the project root
        
    Returns:
        XML string representing the project structure
    """
    config = parse_project_config(project_root)
    if not config:
        return f"Error: Could not parse project config. The path '{project_root}' may not contain a valid Tach project."
    
    # Create the root element
    root = ET.Element("project")
    
    # Add modules
    modules_element = ET.SubElement(root, "modules")
    
    for module in config.all_modules():
        module_element = ET.SubElement(modules_element, "module")
        module_element.set("path", module.path)
        
        if module.layer:
            module_element.set("layer", module.layer)
            
        if module.unchecked:
            module_element.set("unchecked", str(module.unchecked).lower())
        
        # Add dependencies
        if module.depends_on:
            dependencies_element = ET.SubElement(module_element, "dependencies")
            for dependency in module.depends_on:
                dependency_element = ET.SubElement(dependencies_element, "dependency")
                dependency_element.set("path", dependency.path)
                if dependency.deprecated:
                    dependency_element.set("deprecated", str(dependency.deprecated).lower())
        
        # Add visibility
        if module.visibility:
            visibility_element = ET.SubElement(module_element, "visibility")
            for visible_to in module.visibility:
                visible_element = ET.SubElement(visibility_element, "visible-to")
                visible_element.text = visible_to
    
    # Convert to string with pretty formatting
    rough_string = ET.tostring(root, 'utf-8')
    reparsed = minidom.parseString(rough_string)
    return reparsed.toprettyxml(indent="  ")
