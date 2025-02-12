from __future__ import annotations

from tach.parsing.config import parse_project_config
from tach.show import generate_show_report


# right now this is just a smoke test
# this example directory has Python files outside source roots, which has previously caused bugs
def test_many_features_example_dir(example_dir, capfd):
    project_root = example_dir / "many_features"
    project_config = parse_project_config(root=project_root)
    assert project_config is not None

    report = generate_show_report(
        project_root=project_root, project_config=project_config, included_paths=[]
    )
    assert report is not None
