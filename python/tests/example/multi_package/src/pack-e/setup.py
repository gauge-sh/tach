from pathlib import Path

from setuptools import setup


# For this trick, I thank:
# https://stackoverflow.com/a/78056725

root: Path = Path(__file__).parent
pack_a_path: str = (root.parent / "pack-a").as_uri()

if __name__ == "__main__":
    setup(
        install_requires=[f"myorg-pack-a @ {pack_a_path}"],
    )
