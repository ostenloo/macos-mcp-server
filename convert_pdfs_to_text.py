from __future__ import annotations

import os
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Sequence

WORKSPACE_ROOT = Path(__file__).resolve().parent
APPSCRIPTS_DIR = WORKSPACE_ROOT / 'AppScripts'
TOOLS_DIR = WORKSPACE_ROOT / 'tools'
TOOL_BUILD_DIR = TOOLS_DIR / '.build'
PDF2TEXT_SOURCE = TOOLS_DIR / 'pdf2text.swift'


class ConversionError(Exception):
    """Raised when PDF to text conversion fails."""


def log(message: str) -> None:
    print(message)


def warn(message: str) -> None:
    print(f"warning: {message}", file=sys.stderr)


def ensure_tool() -> Path:
    if not PDF2TEXT_SOURCE.exists():
        raise ConversionError(f'Missing converter source at {PDF2TEXT_SOURCE}')

    TOOL_BUILD_DIR.mkdir(parents=True, exist_ok=True)
    binary_path = TOOL_BUILD_DIR / 'pdf2text'

    def needs_rebuild() -> bool:
        if not binary_path.exists():
            return True
        return binary_path.stat().st_mtime < PDF2TEXT_SOURCE.stat().st_mtime

    if needs_rebuild():
        swiftc = shutil.which('swiftc')
        if not swiftc:
            raise ConversionError('swiftc not found; install Xcode toolchain to build converter.')
        log(f"Compiling PDF→text converter → {binary_path}")
        module_cache = TOOL_BUILD_DIR / 'ModuleCache'
        module_cache.mkdir(parents=True, exist_ok=True)
        env = os.environ.copy()
        env.setdefault('SWIFT_MODULE_CACHE_PATH', str(module_cache))
        env.setdefault('CLANG_MODULE_CACHE_PATH', str(module_cache))
        command = [
            swiftc,
            str(PDF2TEXT_SOURCE),
            '-framework',
            'PDFKit',
            '-o',
            str(binary_path),
            '-module-cache-path',
            str(module_cache),
        ]
        result = subprocess.run(
            command,
            cwd=str(TOOLS_DIR),
            env=env,
            capture_output=True,
            text=True,
        )
        if result.returncode != 0:
            error_output = (result.stderr or result.stdout or '').strip()
            if 'this SDK is not supported by the compiler' in error_output or "failed to build module 'Swift'" in error_output:
                raise ConversionError(
                    'Swift compiler/toolchain mismatch: select the full Xcode toolchain via '
                    '`sudo xcode-select -s /Applications/Xcode.app/Contents/Developer`.'
                )
            raise ConversionError(f'Failed to compile converter: {error_output}')

    return binary_path


def convert_pdfs(target_dir: Path) -> None:
    if not target_dir.exists():
        raise ConversionError(f'PDF directory {target_dir} does not exist')

    pdf_files = sorted(target_dir.glob('*.pdf'))
    if not pdf_files:
        warn(f'No PDF files found in {target_dir}')
        return

    converter = ensure_tool()
    text_dir = target_dir / 'text'
    text_dir.mkdir(exist_ok=True)

    for pdf_path in pdf_files:
        txt_path = text_dir / f'{pdf_path.stem}.txt'
        if txt_path.exists() and txt_path.stat().st_mtime >= pdf_path.stat().st_mtime:
            log(f'Skipping {pdf_path.name}: text already up to date')
            continue
        log(f'Extracting text from {pdf_path.name}')
        result = subprocess.run([str(converter), str(pdf_path), str(txt_path)], capture_output=True, text=True)
        if result.returncode != 0:
            warn(f'Failed to convert {pdf_path.name}: {result.stderr or result.stdout}')
            continue

    build_readme(text_dir)


def build_readme(text_dir: Path) -> None:
    readme_path = text_dir / 'README.md'
    entries = []
    for txt_file in sorted(text_dir.glob('*.txt')):
        if txt_file.name == 'README.md':
            continue
        rel_path = txt_file.name
        entries.append(f'- [{txt_file.stem}]({rel_path})')

    lines = [
        '# Extracted AppleScript Dictionaries',
        '',
        'Text exports generated from Objective-C scripting bridge headers.',
        '',
    ]
    if entries:
        lines.extend(entries)
    else:
        lines.append('No text exports available.')
    lines.append('')
    readme_path.write_text('\n'.join(lines), encoding='utf-8')


def main(argv: Sequence[str]) -> int:
    try:
        convert_pdfs(APPSCRIPTS_DIR)
    except ConversionError as exc:
        warn(str(exc))
        return 1
    return 0


if __name__ == '__main__':  # pragma: no cover
    raise SystemExit(main(sys.argv[1:]))
