# Run from the project root with target/debug on PATH.
set -l root (pwd)
set -l temp (mktemp -d)
or exit 1
function cleanup --on-event fish_exit --inherit-variable temp
    command rm -rf -- "$temp"
end
cd "$temp"; or exit 1
git init -q; or exit 1
printf 'docs\n' > README.md
git add README.md; or exit 1
source "$root/completions/git-autocomplete.fish"; or exit 1
source "$root/completions/git-autocomplete.fish"; or exit 1
set -l expected 'docs: add README'
set -l found 0
for entry in (complete -C 'git commit -m ')
    if test (string split \t -- "$entry")[1] = "$expected"
        set found 1
    end
end
test $found -eq 1; or begin; echo 'missing Fish commit suggestion' >&2; exit 1; end
for entry in (complete -C 'git commit --message ')
    if string match -q 'docs: add README*' -- "$entry"
        set found 2
    end
end
test $found -eq 2; or exit 1
for entry in (complete -C 'git commit -m "my message"')
    if string match -q 'docs:*' -- "$entry"
        echo 'Fish overwrote a written message' >&2
        exit 1
    end
end
git -c user.name=Test -c user.email=test@example.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -qm fixture; or exit 1
git branch fixture; or exit 1
set -l branches (complete -C 'git checkout fix')
string match -q 'fixture*' -- $branches; or begin; echo 'Fish branch completion lost' >&2; exit 1; end
# Verify native escaping by reparsing escaped completion as a single token, without executing it.
mkdir quoted; or exit 1
cd quoted; or exit 1
git init -q; or exit 1
printf 'docs\n' > 'Bob\'s $(touch PWN);(x).md'
git add --all
set -l escaped (complete --escape -C 'git commit -m ')
string match -q '*Bob*' -- $escaped; or exit 1
set -l candidate (string split \t -- "$escaped[1]")[1]
set -l tokens
printf '%s\n' "$candidate" | read --tokenize --array tokens
test (count $tokens) -eq 1; or exit 1
test "$tokens[1]" = 'docs: add Bob\'s $(touch PWN);(x)'; or exit 1
test ! -e PWN; or exit 1
echo 'Fish native completion checks passed'
