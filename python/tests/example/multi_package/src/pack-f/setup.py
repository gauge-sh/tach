from pathlib import Path

from setuptools import setup


# For this trick, I thank:
# https://stackoverflow.com/a/78056725

root: Path = Path(__file__).parent
pack_d_path: str = (root.parent / "pack-d").as_uri()
pack_e_path: str = (root.parent / "pack-e").as_uri()

if __name__ == "__main__":
    setup(
        install_requires=[
            f"myorg-pack-d @ {pack_d_path}",
            f"myorg-pack-e @ {pack_e_path}",
        ],
    )
