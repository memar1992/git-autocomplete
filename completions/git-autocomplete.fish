# Source once after Fish's bundled Git completions; definitions are additive.
if not set -q __git_autocomplete_loaded
    # Load bundled/custom Git definitions before adding our message arguments.
    complete -C 'git ' >/dev/null 2>/dev/null
    set -g __git_autocomplete_loaded 1

    function __git_autocomplete_empty_message
        set -l words (commandline -opc)
        test (count $words) -ge 3; or return 1
        contains -- $words[-1] -m --message; or return 1
        set -l command_index 2
        while test "$words[$command_index]" = -C
            set command_index (math $command_index + 2)
        end
        test "$words[$command_index]" = commit; or return 1
        set -l offset (commandline --cursor --current-process)
        set -l process (commandline -p | string collect)
        set -l tail (string sub -s (math $offset + 1) -- "$process" | string trim)
        contains -- "$tail" '' '"' "'"; or return 1
        # Inspect the whole token so text after the cursor cannot be overwritten.
        set -l token (commandline -t)
        if test -n "$token"
            set -l decoded (string unescape -- "$token" 2>/dev/null)
            test $status -eq 0; or return 1
            test -z "$decoded"; or return 1
        end
    end

    complete -c git -s m -l message -r -f -k -n __git_autocomplete_empty_message \
        -a '(command git-autocomplete complete fish -- (commandline -opc) "" 2>/dev/null)'
end
