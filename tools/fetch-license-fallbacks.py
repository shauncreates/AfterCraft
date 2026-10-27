"""Fetch missing license texts only, pinned to dependency source revisions from Cargo."""
import argparse
import json
from pathlib import Path, PurePosixPath
import re
import urllib.request
import subprocess
import hashlib

ROOT = Path(__file__).resolve().parents[1]


def download(url, limit=8 * 1024 * 1024):
    request = urllib.request.Request(url, headers={"User-Agent": "AfterCraft-license-collector"})
    with urllib.request.urlopen(request, timeout=30) as response:
        data = response.read(limit + 1)
    if len(data) > limit:
        raise ValueError("License response exceeds size limit")
    return data


def main(inventory):
    missing = {f'{p["name"]}-{p["version"]}' for p in json.loads(inventory.read_text(encoding="utf-8")) if not p["texts"]}
    trees = {}
    unresolved = set(missing)
    for workspace in (ROOT, ROOT / "native/native-host", ROOT / "native/ofx-helper"):
        md = json.loads(subprocess.check_output(["cargo", "metadata", "--locked", "--format-version", "1"], cwd=workspace, text=True, encoding="utf-8"))
        for package in md["packages"]:
            key = f'{package["name"]}-{package["version"]}'
            if key not in unresolved:
                continue
            folder = Path(package["manifest_path"]).parent
            vcs_file = folder / ".cargo_vcs_info.json"
            repo = (package.get("repository") or "").rstrip("/")
            match = re.match(r"https://github.com/([^/]+/[^/]+)", repo)
            if not vcs_file.exists() or not match:
                print(f"Needs manual license provenance: {key}", flush=True)
                continue
            vcs = json.loads(vcs_file.read_text(encoding="utf-8"))
            revision = vcs["git"]["sha1"]
            repository = match[1].removesuffix(".git")
            tree_key = (repository, revision)
            if tree_key not in trees:
                trees[tree_key] = json.loads(download(f"https://api.github.com/repos/{repository}/git/trees/{revision}?recursive=1"))["tree"]
            package_path = PurePosixPath(vcs.get("path_in_vcs", ""))
            ancestors = {str(p) for p in (package_path, *package_path.parents)} | {"."}
            candidates = []
            for entry in trees[tree_key]:
                path = PurePosixPath(entry["path"])
                if entry["type"] == "blob" and re.match(r"(?i)^(licen[cs]e|copying|notice|unlicense|ofl)", path.name):
                    if str(path.parent) in ancestors or str(path.parent.parent) in ancestors and path.parent.name.lower() in ("licenses", "licences"):
                        candidates.append(str(path))
            if not candidates:
                print(f"No ancestor license file: {key} at {repository}/{revision}", flush=True)
                continue
            target = ROOT / "packaging/licenses" / re.sub(r"[^A-Za-z0-9_.-]", "_", key)
            target.mkdir(parents=True, exist_ok=True)
            manifest = []
            for path in candidates:
                url = f"https://raw.githubusercontent.com/{repository}/{revision}/{path}"
                data = download(url, 1024 * 1024)
                name = path.replace("/", "__")
                (target / name).write_bytes(data)
                manifest.append({"file": name, "source": url, "sha256": hashlib.sha256(data).hexdigest()})
            (target / "sources.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
            unresolved.remove(key)
            print(f"Pinned license texts: {key}", flush=True)
    print("Still unresolved: " + ", ".join(sorted(unresolved)), flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("inventory", type=Path)
    main(parser.parse_args().inventory)
