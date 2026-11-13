#!/usr/bin/env python3
"""Copy exact dependency metadata/licenses into a local package from cargo metadata."""
import json
import argparse
from pathlib import Path
import shutil
import subprocess
import sys
root = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('destination', type=Path)
parser.add_argument('--metadata', type=Path, help='Previously resolved cargo metadata JSON')
args = parser.parse_args()
destination = args.destination
destination.mkdir(parents=True, exist_ok=True)
if args.metadata:
    metadata = json.loads(args.metadata.read_text(encoding='utf-8-sig'))
else:
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
    # Rune embeds font binaries whose redistribution notices live with the assets.
    if package['name'] == 'rune':
        font_notices = list((source / 'assets').glob('*LICENSE*'))
        if not any(path.name == 'Inter-LICENSE.txt' for path in font_notices):
            raise RuntimeError('Rune embedded Inter font notice is missing')
        files.update(font_notices)
    target = destination / label
    for path in sorted(files):
        if path.is_file():
            target.mkdir(exist_ok=True)
            shutil.copyfile(path, target / path.name)
(destination / 'DEPENDENCIES.md').write_text('\n'.join(rows) + '\n', encoding='utf-8')
