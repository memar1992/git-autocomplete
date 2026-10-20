#!/usr/bin/env python3
"""Verify a native release archive, install it in isolation, replace it, and uninstall it."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import zipfile


def check(archive):
    expected = f"{hashlib.sha256(archive.read_bytes()).hexdigest()}  {archive.name}\n"
    assert archive.with_name(archive.name + '.sha256').read_text() == expected, 'checksum mismatch'
    windows = archive.suffix == '.zip'
    suffix = '.zip' if windows else '.tar.gz'
    assert archive.name.endswith(suffix), 'expected a .zip or .tar.gz archive'
    folder = archive.name[:-len(suffix)]
    executable = 'git-autocomplete.exe' if windows else 'git-autocomplete'
    shells = {'zsh': 'zsh', 'bash': 'bash', 'fish': 'fish', 'powershell': 'ps1'}
    names = [executable, *(f'completions/git-autocomplete.{ext}' for ext in shells.values())]
    if windows:
        with zipfile.ZipFile(archive) as bundle:
            contents = {name: bundle.read(f'{folder}/{name}') for name in names}
    else:
        with tarfile.open(archive, 'r:gz') as bundle:
            contents = {name: bundle.extractfile(f'{folder}/{name}').read() for name in names}

    with tempfile.TemporaryDirectory(prefix='git-autocomplete install-') as temp:
        root = Path(temp)
        install = root / 'bin'
        config = root / 'config/git-autocomplete'
        repo = root / 'repository'
        for directory in (install, config, repo):
            directory.mkdir(parents=True)
        program = install / executable
        program.write_bytes(contents[executable])
        program.chmod(0o755)
        env = dict(os.environ, PATH=str(install) + os.pathsep + os.environ['PATH'],
                   GIT_CONFIG_NOSYSTEM='1', GIT_CONFIG_GLOBAL=os.devnull)
        assert Path(shutil.which('git-autocomplete', path=env['PATH'])).resolve() == program.resolve()

        def run(*args):
            return subprocess.check_output(args, cwd=repo, env=env)

        version = run(str(program), 'version')
        assert version.startswith(b'git-autocomplete ')
        for shell, ext in shells.items():
            integration = run(str(program), 'shell', shell)
            assert integration == contents[f'completions/git-autocomplete.{ext}'], 'stale integration'
            (config / f'init.{ext}').write_bytes(integration)
        zsh = shutil.which('zsh')
        if zsh:
            run(zsh, '-f', '-c',
                'autoload -Uz compinit; compinit -D; source "$1"; '
                '[[ ${_comps[git]} == _git_autocomplete_dispatch ]]',
                'test-install', str(config / 'init.zsh'))

        run('git', 'init', '-q')
        (repo / 'README.md').write_text('# Installation fixture\n')
        run('git', 'add', 'README.md')
        index = (repo / '.git/index').read_bytes()
        worktree = (repo / 'README.md').read_bytes()
        status = json.loads(run(str(program), 'status'))
        assert status['offline'] is True and status['staged_files'] == 1
        result = json.loads(run(str(program), 'suggest', '--format', 'json'))
        assert result['suggestions'][0]['message'] == 'docs: add README'
        for shell in shells:
            result = run(str(program), 'complete', shell, '--', 'git', 'commit', '-m', '')
            message = "'docs: add README'" if shell == 'bash' else 'docs: add README'
            assert result.decode().strip() == message

        # No earlier release exists yet; exercise replacement with the current build.
        program.write_bytes(contents[executable])
        assert run(str(program), 'version') == version
        assert run(str(program), 'suggest').decode().strip() == 'docs: add README'
        assert (repo / '.git/index').read_bytes() == index
        assert (repo / 'README.md').read_bytes() == worktree
        program.unlink()
        shutil.rmtree(config)
        assert shutil.which('git-autocomplete', path=str(install)) is None
        assert not config.exists()
    print(f'Checksum, isolated install, integrations, replacement, and uninstall passed: {archive.name}')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('archive', type=Path)
    check(parser.parse_args().archive.resolve())
