#!/usr/bin/env python3
"""Keep locked dependency attribution and available full license texts in dist."""
import json
import shutil
import subprocess
from pathlib import Path

root = Path(__file__).resolve().parent.parent
metadata = json.loads(subprocess.check_output(
    ["cargo", "metadata", "--locked", "--filter-platform", "wasm32-unknown-unknown",
     "--format-version", "1"], cwd=root))
built = {node['id'] for node in metadata['resolve']['nodes']}
fallback_root = root / 'scripts/license-fallbacks'
fallbacks = json.loads((fallback_root / 'manifest.json').read_text())
output = root / "public/licenses/rust"
if output.exists():
    shutil.rmtree(output)
output.mkdir(parents=True, exist_ok=True)
manifest = []
for package in sorted(metadata["packages"], key=lambda p: (p["name"], p["version"])):
    if not package.get("source") or package['id'] not in built:
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
        # Some registry tarballs omit their repository-root notices. These exact
        # texts were obtained from the package's recorded .cargo_vcs_info commit.
        vcs_file = Path(package['manifest_path']).parent / '.cargo_vcs_info.json'
        vcs = json.loads(vcs_file.read_text()) if vcs_file.exists() else {}
        fallback = next((f for f in fallbacks if f['name'] == package['name']
                         and f['version'] == package['version']
                         and f['license'] == package['license']
                         and f['git_sha'] == vcs.get('git', {}).get('sha1')), None)
        if not fallback:
            raise RuntimeError(f'No license text found for {name}; inspect before publishing')
        destination = output / name / fallback['file']
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(fallback_root / fallback['file'], destination)
        (destination.parent / 'SOURCE.txt').write_text(fallback['source_url'] + '\n')
        licenses.append(f'rust/{name}/{destination.name}')
    if package['name'] == 'photocraft-text':
        fonts = Path(package['manifest_path']).parent.parent.parent / 'assets/fonts'
        for source in fonts.rglob('*'):
            if source.is_file() and source.name.upper().startswith(('LICENSE', 'OFL')):
                destination = output / name / ('font-' + source.parent.name + '-' + source.name)
                shutil.copyfile(source, destination)
                licenses.append(f'rust/{name}/{destination.name}')
    manifest.append({"name": package["name"], "version": package["version"],
                     "license": package.get("license"), "source": package["source"], "texts": licenses})
for package in ["mp4box", "webm-muxer", "mp4-muxer"]:
    shutil.copyfile(root / "node_modules" / package / "LICENSE", root / "public/licenses" / f"{package}-LICENSE")
(root / "public/licenses/dependencies.json").write_text(
    json.dumps({"rust": manifest, "npm_runtime": [
        {"name": "mp4box", "version": "2.1.2", "license": "BSD-3-Clause", "text": "mp4box-LICENSE"},
        {"name": "webm-muxer", "version": "5.1.4", "license": "MIT", "text": "webm-muxer-LICENSE"},
        {"name": "mp4-muxer", "version": "5.2.2", "license": "MIT", "text": "mp4-muxer-LICENSE"}]}, indent=2) + "\n")
shutil.copyfile(root / "LICENSE", root / "public/licenses/creative-studio-LICENSE")
print(f'Collected attribution for {len(manifest)} locked Rust packages and three browser libraries.')
