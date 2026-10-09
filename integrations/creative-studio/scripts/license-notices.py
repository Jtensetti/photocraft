#!/usr/bin/env python3
"""Keep locked dependency attribution and available full license texts in dist."""
import json
import shutil
import subprocess
from pathlib import Path

root = Path(__file__).resolve().parent.parent
metadata = json.loads(subprocess.check_output(
    ["cargo", "metadata", "--locked", "--format-version", "1"], cwd=root))
output = root / "public/licenses/rust"
if output.exists():
    shutil.rmtree(output)
output.mkdir(parents=True, exist_ok=True)
manifest = []
for package in sorted(metadata["packages"], key=lambda p: (p["name"], p["version"])):
    if not package.get("source"):
        continue
    name = f'{package["name"]}-{package["version"]}'
    directory = Path(package["manifest_path"]).parent
    if package['name'] == 'alloc-stdlib':
        # This subcrate's published tarball omits the repository-root license.
        # alloc-no-stdlib ships that text and comes from the same repository.
        sibling = next(p for p in metadata['packages'] if p['name'] == 'alloc-no-stdlib')
        if sibling['repository'] != package['repository']:
            raise RuntimeError('Allocator repositories changed; recheck attribution')
        directory = Path(sibling['manifest_path']).parent
    licenses = []
    for ancestor in [directory, *list(directory.parents)[:2]]:
        candidates = sorted(p for p in ancestor.iterdir() if p.is_file()
                            and p.name.upper().startswith(("LICENSE", "COPYING", "NOTICE")))
        if candidates:
            for source in candidates:
                destination = output / name / source.name
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(source, destination)
                licenses.append(f'rust/{name}/{source.name}')
            break
    if not licenses:
        raise RuntimeError(f'No license text found for {name}; inspect before publishing')
    manifest.append({"name": package["name"], "version": package["version"],
                     "license": package.get("license"), "source": package["source"], "texts": licenses})
for package in ["mp4box", "webm-muxer"]:
    shutil.copyfile(root / "node_modules" / package / "LICENSE", root / "public/licenses" / f"{package}-LICENSE")
(root / "public/licenses/dependencies.json").write_text(
    json.dumps({"rust": manifest, "npm_runtime": [
        {"name": "mp4box", "version": "2.1.2", "license": "BSD-3-Clause", "text": "mp4box-LICENSE"},
        {"name": "webm-muxer", "version": "5.1.4", "license": "MIT", "text": "webm-muxer-LICENSE"}]}, indent=2) + "\n")
shutil.copyfile(root / "LICENSE", root / "public/licenses/creative-studio-LICENSE")
print(f'Collected attribution for {len(manifest)} locked Rust packages and two browser libraries.')
