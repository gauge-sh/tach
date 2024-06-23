

# Define the project root and mock project configuration for demonstration
 # Change this to your actual project root

from pathlib import Path
from tach.sync import sync_project

# Import the actual classes based on their definition
# Assuming `tach.core` has a `ProjectConfig` and a `Module` or equivalent
try:
    from tach.core import ProjectConfig, Module  # Adjust if Module is defined elsewhere
except ImportError:
    # If `Module` is defined elsewhere, import it correctly
    # from tach import Module  # Replace with the correct module
    print('Hellow')

# Define the project root and mock project configuration for demonstration
project_root = Path("./")   # Change this to your actual project root

# Mock project configuration
project_config = ProjectConfig(modules=[
    Module(name="module_one"),
    Module(name="module_two"),
    Module(name="<root>")
])

# Run the sync_project function to generate the tach.yml
sync_project(project_root, project_config)

print("sync_project executed successfully and tach.yml is generated.")
