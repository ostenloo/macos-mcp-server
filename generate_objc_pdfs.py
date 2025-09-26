#!/usr/bin/env python3
from __future__ import annotations
"""Generate Objective-C scripting bridge headers as PDFs for scriptable apps."""

import argparse
import os
import plistlib
import shutil
import subprocess
import sys
import tempfile
import unicodedata
from pathlib import Path
from typing import Iterable, List, Optional, Sequence, Tuple

APP_SEARCH_PATHS: Sequence[Path] = (
    Path('/Applications'),
    Path('/System/Applications'),
    Path('/System/Applications/Utilities'),
    Path('/System/Library/CoreServices'),
    Path.home() / 'Applications',
)

WORKSPACE_ROOT = Path(__file__).resolve().parent
TOOLS_DIR = WORKSPACE_ROOT / 'tools'
DEFAULT_OUTPUT_DIR = WORKSPACE_ROOT / 'AppScripts'
TEXT2PDF_SOURCE = TOOLS_DIR / 'text2pdf.swift'
TEXT2PDF_BUILD = TOOLS_DIR / '.build'


class GenerationError(Exception):
    """Raised when an application cannot be processed."""


def log(message: str) -> None:
    print(message)


def warn(message: str) -> None:
    print(f"warning: {message}", file=sys.stderr)


def sanitize_filename(name: str) -> str:
    normalized = unicodedata.normalize('NFKD', name)
    ascii_only = ''.join(ch for ch in normalized if 32 <= ord(ch) < 127)
    cleaned = ascii_only.replace('/', '-').replace(':', '-')
    cleaned = ' '.join(cleaned.split())
    return cleaned if cleaned else 'Untitled'


def unique_name(base: str, used: set[str]) -> str:
    candidate = base
    counter = 2
    while candidate in used:
        candidate = f"{base} ({counter})"
        counter += 1
    used.add(candidate)
    return candidate


def ensure_text2pdf() -> Path:
    if not TEXT2PDF_SOURCE.exists():
        raise GenerationError(f'Missing converter source at {TEXT2PDF_SOURCE}')

    TEXT2PDF_BUILD.mkdir(parents=True, exist_ok=True)
    binary_path = TEXT2PDF_BUILD / 'text2pdf'

    def needs_rebuild() -> bool:
        if not binary_path.exists():
            return True
        return binary_path.stat().st_mtime < TEXT2PDF_SOURCE.stat().st_mtime

    if needs_rebuild():
        swiftc = shutil.which('swiftc')
        if not swiftc:
            raise GenerationError('swiftc not found; install Xcode command-line tools or Swift toolchain.')
        log(f"Compiling text converter → {binary_path}")
        module_cache = TEXT2PDF_BUILD / 'ModuleCache'
        module_cache.mkdir(parents=True, exist_ok=True)
        env = os.environ.copy()
        env.setdefault('SWIFT_MODULE_CACHE_PATH', str(module_cache))
        env.setdefault('CLANG_MODULE_CACHE_PATH', str(module_cache))
        command = [
            swiftc,
            str(TEXT2PDF_SOURCE),
            '-o',
            str(binary_path),
            '-module-cache-path',
            str(module_cache),
        ]
        result = subprocess.run(
            command,
            cwd=str(TEXT2PDF_SOURCE.parent),
            env=env,
            capture_output=True,
            text=True,
        )
        if result.returncode != 0:
            error_output = (result.stderr or result.stdout or '').strip()
            if 'module cache' in error_output or 'Operation not permitted' in error_output:
                raise GenerationError(
                    'Swift compiler could not write its module cache. Ensure the sandbox allows '
                    'writes to the repository or run this script outside the restricted environment.'
                )
            if "failed to build module 'Swift'" in error_output or 'this SDK is not supported by the compiler' in error_output:
                raise GenerationError(
                    'Swift compiler/toolchain mismatch: install or select the full Xcode toolchain '
                    'via `sudo xcode-select -s /Applications/Xcode.app/Contents/Developer`.'
                )
            if "redefinition of module 'SwiftBridging'" in error_output:
                raise GenerationError(
                    'Swift headers from the Command Line Tools are conflicting. Switching to the full '
                    'Xcode toolchain usually resolves the duplicate module definition.'
                )
            raise GenerationError(f'Failed to compile text converter: {error_output}')
    return binary_path


def load_info_plist(app_path: Path) -> Optional[dict]:
    info_path = app_path / 'Contents' / 'Info.plist'
    if not info_path.exists():
        return None
    try:
        with info_path.open('rb') as handle:
            return plistlib.load(handle)
    except Exception as exc:  # pragma: no cover
        warn(f'Failed to read Info.plist for {app_path}: {exc}')
        return None


def find_script_resources(app_path: Path, info: dict, temp_dir: Path) -> Optional[List[Path]]:
    scripting_data = info.get('OSAScriptingDefinition')
    if scripting_data:
        resource_root = app_path / 'Contents' / 'Resources'
        if isinstance(scripting_data, bytes):
            sdef_path = temp_dir / 'dictionary.sdef'
            sdef_path.write_bytes(scripting_data)
            return [sdef_path]

        if isinstance(scripting_data, str):
            candidate = Path(scripting_data)
            if not candidate.is_absolute():
                candidate = resource_root / candidate
            if candidate.exists():
                return [candidate]

            warn(
                f"{app_path.stem}: Info.plist references OSAScriptingDefinition '{scripting_data}', "
                'but the resource was not found.'
            )
            return None

    resource_root = app_path / 'Contents' / 'Resources'
    if resource_root.exists():
        sdef_candidates = list(resource_root.rglob('*.sdef'))
        if sdef_candidates:
            # Prefer English localization if available
            preferred = sorted(
                sdef_candidates,
                key=lambda path: (0 if '.lproj' not in path.parts else (0 if 'en.lproj' in path.parts else 1), len(path.parts))
            )
            return [preferred[0]]

        suite_candidates = list(resource_root.rglob('*.scriptSuite'))
        if suite_candidates:
            suite_candidates.sort()
            suite = suite_candidates[0]
            term = suite.with_suffix('.scriptTerminology')
            if term.exists():
                return [suite, term]

    return None


def run_sdp(inputs: Sequence[Path], basename: str, work_dir: Path) -> Path:
    sdp = shutil.which('sdp')
    if not sdp:
        raise GenerationError('sdp not found; install Xcode to access scripting bridge tools.')

    command = [sdp, '-fh', '--basename', basename]
    command.extend(str(path) for path in inputs)
    result = subprocess.run(command, cwd=str(work_dir), capture_output=True, text=True)
    if result.returncode != 0:
        raise GenerationError(
            f"sdp failed for {basename}: {result.stderr.strip() or result.stdout.strip() or 'unknown error'}"
        )

    header_path = work_dir / f'{basename}.h'
    if not header_path.exists():
        raise GenerationError(f'sdp did not produce expected header {header_path}')
    return header_path


def convert_to_pdf(converter: Path, header_path: Path, output_path: Path) -> None:
    output_path.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run([str(converter), str(header_path), str(output_path)], check=True)


def iter_applications(paths: Iterable[Path]) -> Iterable[Tuple[Path, dict]]:
    seen: set[str] = set()
    for base in paths:
        if not base.exists():
            continue
        for app in base.rglob('*.app'):
            if not app.is_dir():
                continue
            info = load_info_plist(app)
            if info is None:
                continue
            bundle_id = info.get('CFBundleIdentifier')
            key = bundle_id or str(app.resolve())
            if key in seen:
                continue
            seen.add(key)
            yield app, info


def generate_pdfs(output_dir: Path, scope: str) -> None:
    output_dir.mkdir(parents=True, exist_ok=True)
    converter = ensure_text2pdf()
    used_names: set[str] = {path.stem for path in output_dir.glob('*.pdf')}

    if scope == 'user':
        search_paths = [Path('/Applications'), Path.home() / 'Applications']
    elif scope == 'system':
        search_paths = [
            Path('/System/Applications'),
            Path('/System/Applications/Utilities'),
            Path('/System/Library/CoreServices'),
        ]
    else:
        search_paths = list(APP_SEARCH_PATHS)

    for app_path, info in iter_applications(search_paths):
        app_name = info.get('CFBundleDisplayName') or info.get('CFBundleName') or app_path.stem
        safe_name = sanitize_filename(app_name)
        existing_pdf = output_dir / f'{safe_name}.pdf'
        if existing_pdf.exists():
            log(f'Skipping {app_name}: {existing_pdf.relative_to(WORKSPACE_ROOT)} already exists')
            continue

        with tempfile.TemporaryDirectory() as tmp_dir_str:
            tmp_dir = Path(tmp_dir_str)
            inputs = find_script_resources(app_path, info, tmp_dir)
            if not inputs:
                warn(f'Skipping {app_name}: no scripting definition found')
                continue
            basename = ''.join(ch for ch in safe_name if ch.isalnum()) or 'Dictionary'
            try:
                header_path = run_sdp(inputs, basename, tmp_dir)
            except GenerationError as exc:
                warn(f'{app_name}: {exc}')
                continue

            pdf_name = unique_name(safe_name, used_names)
            pdf_path = output_dir / f'{pdf_name}.pdf'
            try:
                convert_to_pdf(converter, header_path, pdf_path)
                log(f'Wrote {pdf_path.relative_to(WORKSPACE_ROOT)}')
            except subprocess.CalledProcessError as exc:
                warn(f'Failed to convert header for {app_name}: {exc}')


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=DEFAULT_OUTPUT_DIR, help='Destination directory for PDFs')
    scope_group = parser.add_mutually_exclusive_group()
    scope_group.add_argument('--user-only', action='store_true', help='Only process /Applications and ~/Applications')
    scope_group.add_argument('--system-only', action='store_true', help='Only process system application bundles')
    return parser.parse_args(argv)


def main(argv: Sequence[str]) -> int:
    args = parse_args(argv)
    if args.user_only:
        scope = 'user'
    elif args.system_only:
        scope = 'system'
    else:
        scope = 'all'
    try:
        generate_pdfs(args.output, scope=scope)
    except GenerationError as exc:
        warn(str(exc))
        return 1
    return 0


if __name__ == '__main__':  # pragma: no cover
    raise SystemExit(main(sys.argv[1:]))
