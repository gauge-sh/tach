from pathlib import Path

from setuptools import find_namespace_packages, setup


# For this trick, I thank:
# https://stackoverflow.com/a/78056725

root: Path = Path(__file__).parent
pack_b_path: str = (root.parent / "pack-b").as_uri()

if __name__ == "__main__":
    setup(
        install_requires=[f"myorg-pack-b @ {pack_b_path}"],
        packages=find_namespace_packages(include=["myorg.*"]),
    )
