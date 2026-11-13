#!/usr/bin/env python3
"""Copy exact dependency metadata/licenses into a local package from cargo metadata."""
import json
from pathlib import Path
import shutil
import subprocess
import sys
root = Path(__file__).resolve().parent.parent
destination = Path(sys.argv[1])
destination.mkdir(parents=True, exist_ok=True)
metadata = json.loads(subprocess.check_output([sys.executable, str(root / 'scripts/cargo-local.py'),
                                             'metadata', '--locked', '--offline', '--format-version', '1'], cwd=root))
workspace = set(metadata['workspace_members'])
rows = ['# Third-party dependencies', '', 'Generated from the locked source metadata. Optional/other-platform dependencies are included.', '']
for package in sorted(metadata['packages'], key=lambda p: (p['name'], p['version'])):
    if package['id'] in workspace:
        continue
    label = f"{package['name']}-{package['version']}"
    rows.append(f"- {label}: {package.get('license') or 'see license file'}; {package.get('repository') or package.get('source') or ''}")
    source = Path(package['manifest_path']).parent
    files = set(source.glob('LICENSE*')) | set(source.glob('LICENCE*')) | set(source.glob('COPYING*')) | set(source.glob('NOTICE*'))
    if package.get('license_file'):
        files.add(source / package['license_file'])
    # Workspace-inherited licenses (Rune) can live above the crate directory.
    if not any(path.is_file() for path in files):
        for parent in list(source.parents)[:3]:
            candidates = list(parent.glob('LICENSE*'))
            if candidates:
                files.update(candidates)
                break
    target = destination / label
    for path in sorted(files):
        if path.is_file():
            target.mkdir(exist_ok=True)
            shutil.copyfile(path, target / path.name)
(destination / 'DEPENDENCIES.md').write_text('\n'.join(rows) + '\n')
