import unittest
from pathlib import Path
from tach.sync import sync_project, reorder_modules
from tach.core import ProjectConfig, Module

class TestSync(unittest.TestCase):
    def test_reorder_modules(self):
        project_config = ProjectConfig(modules=[
            Module(name="module_one"),
            Module(name="module_two"),
            Module(name="<root>")
        ])
        reorder_modules(project_config)
        self.assertEqual(
            [module.name for module in project_config.modules],
            ["module_one", "module_two", "<root>"]
        )

        project_config = ProjectConfig(modules=[
            Module(name="<root>"),
            Module(name="module_one"),
            Module(name="module_two")
        ])
        reorder_modules(project_config)
        self.assertEqual(
            [module.name for module in project_config.modules],
            ["module_one", "module_two", "<root>"]
        )

    def test_sync_project(self):
        # Mock paths and project config
        project_root = Path("/fake/project/root")
        project_config = ProjectConfig(modules=[
            Module(name="module_one"),
            Module(name="module_two"),
            Module(name="<root>")
        ])
        
        # Perform the sync project action
        sync_project(project_root, project_config)
        
        # Ensure <root> is last
        self.assertEqual(
            [module.name for module in project_config.modules],
            ["module_one", "module_two", "<root>"]
        )

if __name__ == '__main__':
    unittest.main()
