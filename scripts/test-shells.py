#!/usr/bin/env python3
"""Exercise real Readline/ZLE Tab insertion, using a binary or an explicit adapter-only stub."""
import argparse
import os
from pathlib import Path
import select
import shlex
import shutil
import subprocess
import sys
import tempfile
import time
import unittest

ROOT = Path(__file__).resolve().parents[1]
MESSAGE = "docs: add Bob's $(touch PWN);`touch PWN` \"q\"&(x)!x\\y"


class ShellTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory(prefix="git-autocomplete-shell-")
        cls.base = Path(cls.temp.name)
        cls.bin = cls.base / "bin"
        cls.bin.mkdir()
        executable = cls.bin / "git-autocomplete"
        if OPTIONS.stub:
            executable.write_text(
                "#!/usr/bin/env python3\n"
                "import os, shlex, sys\n"
                "a = sys.argv[1:]\n"
                "if len(a) < 7 or a[0] != 'complete' or a[-1] != '' or a[-2] not in ('-m', '--message') or 'commit' not in a:\n"
                "    sys.exit(2)\n"
                "m = os.environ['GA_MESSAGE']\n"
                "print(shlex.quote(m) if a[1] == 'bash' else m)\n"
            )
            executable.chmod(0o755)
        else:
            binary = Path(OPTIONS.binary).resolve()
            if not binary.is_file():
                raise RuntimeError(f"binary not found: {binary}; build first or use --stub for adapter tests")
            executable.symlink_to(binary)

    @classmethod
    def tearDownClass(cls):
        cls.temp.cleanup()

    def run_tab(self, shell, command, expected, inside=False, real_git_completion=False, cursor_left=0,
                expected_buffer=None, source=None):
        import pty
        if not shutil.which(shell):
            self.skipTest(f"{shell} not installed")
        repo = Path(tempfile.mkdtemp(dir=self.base))
        capture = repo / "args"
        buffer_capture = repo / "buffer"
        subprocess.run(["git", "init", "-q", str(repo)], check=True)
        # This path deliberately contains shell operators. Git only sees a filename.
        if source is None:
            filename = MESSAGE[len("docs: add "):] + ".md"
            (repo / filename).write_text("documentation\n")
        else:
            path = repo / "src/auth/login.ts"
            path.parent.mkdir(parents=True)
            path.write_text(source)
        subprocess.run(["git", "-C", str(repo), "add", "--all"], check=True)
        completion_script = None
        if real_git_completion:
            subprocess.run(["git", "-C", str(repo), "-c", "user.name=Test", "-c", "user.email=test@example.invalid",
                            "-c", "core.hooksPath=/dev/null", "-c", "commit.gpgsign=false", "commit", "-qm", "fixture"], check=True)
            subprocess.run(["git", "-C", str(repo), "branch", "fixture"], check=True)
            if shell == "bash":
                git_exec = Path(subprocess.check_output(["git", "--exec-path"], text=True).strip())
                candidates = [Path("/usr/share/bash-completion/completions/git"),
                              git_exec.parent.parent / "share/git-core/git-completion.bash"]
                completion_script = next((path for path in candidates if path.is_file()), None)
                if completion_script is None:
                    self.skipTest("Git Bash completion script not installed")
        env = dict(os.environ, PATH=str(self.bin) + os.pathsep + os.environ["PATH"],
                   TERM="xterm", GA_CAPTURE=str(capture), GA_BUFFER=str(buffer_capture),
                   GA_MESSAGE=MESSAGE, HISTFILE=os.devnull)
        pid, fd = pty.fork()
        if pid == 0:
            os.chdir(repo)
            os.environ.clear()
            os.environ.update(env)
            argv = [shell, "--noprofile", "--norc", "-i"] if shell == "bash" else [shell, "-f", "-i"]
            os.execvp(shell, argv)
        transcript = bytearray()

        def drain(seconds):
            end = time.monotonic() + seconds
            while time.monotonic() < end:
                if select.select([fd], [], [], max(0, end - time.monotonic()))[0]:
                    try:
                        chunk = os.read(fd, 65536)
                    except OSError:
                        break
                    if not chunk:
                        break
                    transcript.extend(chunk)

        try:
            drain(0.2)
            script = shlex.quote(str(ROOT / "completions" / f"git-autocomplete.{shell}"))
            setup = 'git() { if [[ $1 == commit || $1 == checkout || ( $1 == branch && $2 == -m ) ]]; then printf "%s\\0" "$@" > "$GA_CAPTURE"; else command git "$@"; fi; }; '
            if shell == "zsh":
                setup += "autoload -Uz compinit; compinit -D; "
                if not real_git_completion:
                    setup += "_original() { compadd -- branch_one; }; compdef _original git; "
                setup += "unsetopt BEEP; "
                if expected_buffer is not None:
                    setup += '_ga_buffer() { print -rn -- "$BUFFER" > "$GA_BUFFER"; }; zle -N _ga_buffer; bindkey "^X^B" _ga_buffer; '
                if source is not None:
                    setup += 'bindkey "^I" menu-complete; '
            else:
                if real_git_completion:
                    setup += f"source {shlex.quote(str(completion_script))}; "
                else:
                    setup += "_original() { COMPREPLY=(branch_one); }; complete -F _original git; "
                setup += "bind 'set bell-style none'; "
            setup += f"source {script}; source {script}; PS1='GA> '; printf '\\107A_READY\\n'\n"
            os.write(fd, setup.encode())
            deadline = time.monotonic() + 10
            while b"GA_READY\r\n" not in transcript and time.monotonic() < deadline:
                drain(0.1)
            self.assertIn(b"GA_READY\r\n", transcript, transcript.decode(errors="replace"))
            os.write(fd, command.encode())
            left = 1 if inside else cursor_left
            if left:
                os.write(fd, b"\x1b[D" * left)
            os.write(fd, b"\t")
            drain(0.5)
            self.assertFalse(capture.exists(), "Tab executed the command")
            if expected_buffer is not None:
                os.write(fd, b"\x18\x02")
                deadline = time.monotonic() + 2
                while not buffer_capture.exists() and time.monotonic() < deadline:
                    drain(0.1)
                self.assertTrue(buffer_capture.exists(), transcript.decode(errors="replace"))
                self.assertEqual(buffer_capture.read_text().rstrip(), expected_buffer)
            if left:
                os.write(fd, b"\x05")
            os.write(fd, b"\n")
            deadline = time.monotonic() + 5
            while not capture.exists() and time.monotonic() < deadline:
                drain(0.1)
            self.assertTrue(capture.exists(), transcript.decode(errors="replace"))
            actual = capture.read_bytes().split(b"\0")[:-1]
            self.assertEqual([a.decode() for a in actual], expected, transcript.decode(errors="replace"))
            self.assertFalse((repo / "PWN").exists(), "completion allowed shell injection")
        finally:
            os.write(fd, b"\x03exit\n")
            drain(0.1)
            os.close(fd)
            os.waitpid(pid, 0)

    def test_bash_bare(self):
        self.run_tab("bash", "git commit -m ", ["commit", "-m", MESSAGE])

    def test_zsh_bare(self):
        self.run_tab("zsh", "git commit -m ", ["commit", "-m", MESSAGE])

    def test_zsh_inserts_double_quotes(self):
        escaped = MESSAGE
        for char in ('\\', '"', '$', '`', '!'):
            escaped = escaped.replace(char, '\\' + char)
        self.run_tab("zsh", "git commit -m ", ["commit", "-m", MESSAGE],
                     expected_buffer='git commit -m "' + escaped + '"')

    def test_zsh_selects_first_of_multiple_quoted_suggestions(self):
        if OPTIONS.stub:
            self.skipTest("multiple suggestions require the Rust engine")
        message = "feat(auth): add validateLogin function"
        self.run_tab("zsh", "git commit -m ", ["commit", "-m", message],
                     expected_buffer='git commit -m "' + message + '"',
                     source="export function validateLogin(user) { return !!user; }\n")

    def test_bash_long(self):
        self.run_tab("bash", "git commit --message ", ["commit", "--message", MESSAGE])

    def test_zsh_long(self):
        self.run_tab("zsh", "git commit --message ", ["commit", "--message", MESSAGE])

    def test_zsh_empty_quotes(self):
        self.run_tab("zsh", 'git commit -m ""', ["commit", "-m", MESSAGE], inside=True)

    def test_bash_empty_quotes_are_preserved(self):
        self.run_tab("bash", 'git commit -m ""', ["commit", "-m", ""], inside=True)

    def test_written_messages(self):
        for shell in ("bash", "zsh"):
            with self.subTest(shell=shell):
                self.run_tab(shell, 'git commit -m "my message"', ["commit", "-m", "my message"])

    def test_written_message_after_cursor(self):
        for shell in ("bash", "zsh"):
            with self.subTest(shell=shell):
                self.run_tab(shell, 'git commit -m  "my message"', ["commit", "-m", "my message"], cursor_left=13)

    def test_other_subcommands_delegate(self):
        for shell in ("bash", "zsh"):
            with self.subTest(shell=shell):
                self.run_tab(shell, "git checkout br", ["checkout", "branch_one"])

    def test_other_message_flags_delegate(self):
        for shell in ("bash", "zsh"):
            with self.subTest(shell=shell):
                self.run_tab(shell, "git branch -m br", ["branch", "-m", "branch_one"])

    def test_real_git_bash_completion(self):
        self.run_tab("bash", "git checkout fix", ["checkout", "fixture"], real_git_completion=True)

    def test_real_git_zsh_completion(self):
        self.run_tab("zsh", "git checkout fix", ["checkout", "fixture"], real_git_completion=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", default=str(ROOT / "target/debug/git-autocomplete"))
    parser.add_argument("--stub", action="store_true", help="test adapters only without a compiled Rust engine")
    OPTIONS, rest = parser.parse_known_args()
    if os.name == "nt":
        sys.exit("Use scripts/test-powershell.ps1 on Windows; PTY tests require Unix.")
    unittest.main(argv=[sys.argv[0]] + rest)
