import pytest
from modguard.core import (
    ModuleConfig,
    ModuleTrie,
    ModuleNode,
    ProjectConfig,
    ScopeDependencyRules,
)
from modguard.check import check_import


@pytest.fixture
def test_config() -> ModuleConfig:
    return ModuleConfig(tags=["test"], strict=False)


@pytest.fixture
def project_config() -> ProjectConfig:
    return ProjectConfig(
        dependency_rules={
            "domain_one": ScopeDependencyRules(
                depends_on=["domain_one", "domain_three"]
            ),
            "domain_two": ScopeDependencyRules(depends_on=["domain_one"]),
            "domain_three": ScopeDependencyRules(depends_on=[]),
        }
    )


@pytest.fixture
def module_trie() -> ModuleTrie:
    return ModuleTrie(
        root=ModuleNode(
            is_end_of_path=False,
            full_path="",
            config=None,
            children={
                "domain_one": ModuleNode(
                    is_end_of_path=True,
                    full_path="domain_one",
                    config=ModuleConfig(tags=["domain_one"], strict=True),
                    children={
                        "subdomain": ModuleNode(
                            is_end_of_path=True,
                            full_path="domain_one.subdomain",
                            config=ModuleConfig(tags=["domain_one"], strict=True),
                            children={},
                        )
                    },
                ),
                "domain_two": ModuleNode(
                    is_end_of_path=True,
                    full_path="domain_two",
                    config=ModuleConfig(tags=["domain_two"], strict=False),
                    children={
                        "subdomain": ModuleNode(
                            is_end_of_path=True,
                            full_path="domain_two.subdomain",
                            config=ModuleConfig(tags=["domain_two"], strict=False),
                            children={},
                        )
                    },
                ),
                "domain_three": ModuleNode(
                    is_end_of_path=True,
                    full_path="domain_three",
                    config=ModuleConfig(tags=["domain_three"], strict=False),
                    children={},
                ),
            },
        )
    )


@pytest.mark.parametrize(
    "file_mod_path,import_mod_path,expected_result",
    [
        ("domain_one", "domain_one", True),
        ("domain_one", "domain_one.subdomain", True),
        ("domain_one", "domain_one.core", True),
        ("domain_one", "domain_three", True),
        ("domain_two", "domain_one", True),
        ("domain_two.subdomain", "domain_one", True),
        ("domain_two", "external", True),
        ("external", "external", True),
        ("domain_three", "domain_one", False),
        ("domain_two", "domain_one.core", False),
        ("domain_two.subdomain", "domain_one.core", False),
        ("domain_two", "domain_three", False),
        ("domain_two", "domain_two.subdomain", False),
        ("external", "domain_three", False),
    ],
)
def test_check_import(
    project_config, module_trie, file_mod_path, import_mod_path, expected_result
):
    result = check_import(
        project_config=project_config,
        module_trie=module_trie,
        file_mod_path=file_mod_path,
        import_mod_path=import_mod_path,
    )
    assert result.ok == expected_result
