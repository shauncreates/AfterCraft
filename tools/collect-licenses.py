"""Collect dependency license texts and a path-free inventory for binary distributions."""
import argparse
import json
from pathlib import Path
import re
import shutil
import subprocess


def collect(destination):
    root = Path(__file__).resolve().parents[1]
    destination.mkdir(parents=True, exist_ok=True)
    inventory = {}
    host = next(line.removeprefix("host: ") for line in subprocess.check_output(
        ["rustc", "-vV"], text=True, encoding="utf-8").splitlines() if line.startswith("host: "))
    for workspace in (root, root / "native/native-host", root / "native/ofx-helper"):
        result = subprocess.run(
            ["cargo", "metadata", "--locked", "--filter-platform", host, "--format-version", "1"],
            cwd=workspace, check=True, capture_output=True, text=True, encoding="utf-8",
        )
        metadata = json.loads(result.stdout)
        selected = {node["id"] for node in metadata["resolve"]["nodes"]}
        for package in metadata["packages"]:
            if package["id"] not in selected:
                continue
            key = f'{package["name"]}-{package["version"]}'
            if key in inventory:
                continue
            folder = Path(package["manifest_path"]).parent
            target = destination / re.sub(r"[^A-Za-z0-9_.-]", "_", key)
            def license_texts(directory):
                return [p for p in directory.iterdir() if p.is_file() and
                        p.name.upper().startswith(("LICENSE", "LICENCE", "COPYING", "NOTICE", "UNLICENSE", "OFL", "UFL"))]
            texts = license_texts(folder)
            # Git dependencies often put shared licenses above individual workspace crates.
            if not texts and (package.get("source") or "").startswith("git+"):
                for parent in list(folder.parents)[:5]:
                    texts = license_texts(parent)
                    if texts or (parent / ".git").exists():
                        break
            for child in folder.iterdir():
                if child.is_dir() and child.name.lower() in ("licenses", "licences", "fonts"):
                    texts.extend(p for p in child.iterdir() if p.is_file() and p.suffix.lower() == ".txt")
                    texts.extend(p for p in license_texts(child) if p not in texts)
            # Workspace packages can inherit their license from their workspace root.
            if not texts and folder.is_relative_to(root):
                texts = [root / "LICENSE-MIT", root / "LICENSE-APACHE"]
            fallback = root / "packaging/licenses" / re.sub(r"[^A-Za-z0-9_.-]", "_", key)
            if not texts and fallback.is_dir():
                texts = [p for p in fallback.iterdir() if p.is_file()]
            if not texts:
                raise ValueError(f"Missing dependency license text: {key}; prepare its pinned supplement before packaging")
            target.mkdir(exist_ok=True)
            for text in texts:
                shutil.copy2(text, target / text.name)
            inventory[key] = {"name": package["name"], "version": package["version"], "authors": package.get("authors", []),
                              "license": package.get("license"), "repository": package.get("repository"),
                              "sourceArchive": f'https://crates.io/api/v1/crates/{package["name"]}/{package["version"]}/download' if (package.get("source") or "").startswith("registry+") else package.get("source"),
                              "texts": [p.name for p in texts]}
    (destination / "inventory.json").write_text(
        json.dumps(sorted(inventory.values(), key=lambda p: (p["name"], p["version"])), indent=2) + "\n",
        encoding="utf-8",
    )
    print(f"Collected {len(inventory)} dependency license entries")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("destination", type=Path)
    collect(parser.parse_args().destination)
