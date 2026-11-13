#!/usr/bin/env python3
"""Validate the distributable ZIP and optionally exercise its extracted Windows binary."""
import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import struct
import subprocess
import tempfile
import zipfile


def pe_imports(data):
    if data[:2] != b'MZ':
        raise ValueError('ATHANOR.exe has no DOS executable header')
    pe = struct.unpack_from('<I', data, 0x3c)[0]
    if data[pe:pe + 4] != b'PE\0\0':
        raise ValueError('ATHANOR.exe has no PE signature')
    machine, sections = struct.unpack_from('<HH', data, pe + 4)
    optional_size = struct.unpack_from('<H', data, pe + 20)[0]
    optional = pe + 24
    if machine != 0x8664 or struct.unpack_from('<H', data, optional)[0] != 0x20b:
        raise ValueError('ATHANOR.exe must be an AMD64 PE32+ executable')
    section_table = optional + optional_size

    def offset(rva):
        for index in range(sections):
            entry = section_table + index * 40
            size, address, raw_size, raw = struct.unpack_from('<IIII', data, entry + 8)
            if address <= rva < address + max(size, raw_size):
                return raw + rva - address
        raise ValueError(f'Unmapped PE RVA {rva:#x}')

    def dll_name(rva):
        start = offset(rva)
        end = data.index(b'\0', start)
        return data[start:end].decode('ascii').lower()

    imports = set()
    import_rva = struct.unpack_from('<I', data, optional + 112 + 8)[0]
    if import_rva:
        cursor = offset(import_rva)
        while any(data[cursor:cursor + 20]):
            imports.add(dll_name(struct.unpack_from('<I', data, cursor + 12)[0]))
            cursor += 20
    delay_rva = struct.unpack_from('<I', data, optional + 112 + 13 * 8)[0]
    if delay_rva:
        cursor = offset(delay_rva)
        while any(data[cursor:cursor + 32]):
            attributes, name = struct.unpack_from('<II', data, cursor)
            if not attributes & 1:
                raise ValueError('Unsupported absolute delayed import')
            imports.add(dll_name(name))
            cursor += 32
    # These require redistributables on a clean PC. Our package uses a static CRT.
    forbidden = [name for name in imports if name.startswith(('vcruntime', 'msvcp', 'concrt'))]
    if forbidden:
        raise ValueError(f'Unbundled Visual C++ runtime imports: {forbidden}')
    return sorted(imports)


def smoke(executable, scratch):
    roots = scratch / 'search'
    roots.mkdir()
    (roots / 'AthanorPortableFixture.txt').write_text('portable smoke fixture', encoding='utf-8')
    (roots / 'AthanorPortableFolder').mkdir()
    settings = scratch / 'data'
    settings.mkdir()
    config = json.dumps({'search_roots': [str(roots)], 'opacity': 0.71}).encode()
    config_file = settings / 'config.json'
    config_file.write_bytes(config)
    env = dict(os.environ, ATHANOR_DATA_DIR=str(settings))
    checks = [
        ('sqrt(144)+2^5', '44'),
        ('32 F to C', '0 °C'),
        ('2026-11-11 + 45 days', 'Dec 26, 2026'),
        ('25 usd to usd', '25 USD'),
        ('help', 'Calculator'),
        ('help calculator', 'Calculator'),
        ('help units', 'Measurement conversions'),
        ('help calendar', 'Calendar'),
        ('help currency', 'Currency'),
        ('help search', 'Applications and files'),
        ('AthanorPortableFixture.txt', 'AthanorPortableFixture.txt'),
        ('AthanorPortableFolder', 'AthanorPortableFolder'),
    ]
    for query, expected in checks:
        result = subprocess.run([str(executable), '--query', query, '--json'],
                                env=env, cwd=executable.parent, capture_output=True,
                                timeout=60, check=True)
        reply = json.loads(result.stdout)
        titles = [item['title'] for item in reply['items']]
        if expected not in titles:
            raise ValueError(f'Query {query!r}: expected {expected!r}, received {titles!r}')
        if config_file.read_bytes() != config:
            raise ValueError('CLI smoke changed existing user configuration')
    return [query for query, _ in checks]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('archive', type=Path)
    parser.add_argument('--source-commit', required=True)
    parser.add_argument('--smoke', action='store_true')
    parser.add_argument('--report', type=Path)
    args = parser.parse_args()
    if not re.fullmatch(r'[0-9a-f]{40}', args.source_commit):
        raise ValueError('Expected a full source commit SHA')
    required = {'ATHANOR.exe', 'BUILD_INFO.json', 'WINDOWS-USAGE.md',
                'assets/icon.svg', 'licenses/DEPENDENCIES.md'}
    with zipfile.ZipFile(args.archive) as archive:
        names = archive.namelist()
        if len(set(name.casefold() for name in names)) != len(names):
            raise ValueError('Archive has duplicate paths')
        for name in names:
            path = PurePosixPath(name)
            if path.is_absolute() or '..' in path.parts or '\\' in name or ':' in name:
                raise ValueError(f'Unsafe archive path {name!r}')
        if not required.issubset(names):
            raise ValueError(f'Missing distribution files: {required.difference(names)}')
        if not any(name.startswith('licenses/') and '/LICENSE' in name.upper() for name in names):
            raise ValueError('Dependency license texts are missing')
        if not any(name.startswith('licenses/rune-') and name.endswith('/Inter-LICENSE.txt') for name in names):
            raise ValueError('Rune embedded Inter font notice is missing')
        info = json.loads(archive.read('BUILD_INFO.json').decode('utf-8-sig'))
        if (info['source_commit'] != args.source_commit
                or info['target'] != 'x86_64-pc-windows-msvc'
                or info['signed'] or info['windows_gui_runtime_verified']
                or not info['static_crt']):
            raise ValueError('Unexpected build identity or preview verification claims')
        imports = pe_imports(archive.read('ATHANOR.exe'))
        checks = []
        if args.smoke:
            if os.name != 'nt':
                raise ValueError('--smoke requires native Windows')
            system32 = Path(os.environ['SystemRoot']) / 'System32'
            for dll in imports:
                if not (system32 / dll).is_file() and not dll.startswith(('api-ms-win-', 'ext-ms-')):
                    raise ValueError(f'Required DLL is not supplied by Windows: {dll}')
            with tempfile.TemporaryDirectory(prefix='athanor-portable-') as directory:
                scratch = Path(directory)
                extracted = scratch / 'package'
                archive.extractall(extracted)
                checks = smoke(extracted / 'ATHANOR.exe', scratch)
    result = {'source_commit': args.source_commit, 'machine': 'AMD64',
              'size_bytes': args.archive.stat().st_size,
              'sha256': hashlib.sha256(args.archive.read_bytes()).hexdigest(),
              'imports': imports, 'smoke_queries': checks,
              'windows_gui_runtime_verified': False}
    if args.report:
        args.report.write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
