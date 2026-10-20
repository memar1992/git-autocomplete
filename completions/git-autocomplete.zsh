# Source after compinit and any Git completion plugin.
if (( ! $+functions[compdef] )); then
  print -u2 -- 'git-autocomplete: run autoload -Uz compinit && compinit before sourcing this script'
  return 1
fi
if [[ ${_git_autocomplete_loaded:-} == 1 ]]; then
  return 0
fi
typeset -g _git_autocomplete_loaded=1
typeset -g _git_autocomplete_previous=${_comps[git]:-_git}

_git_autocomplete_dispatch() {
  local -a args suggestions
  local output command_index=2
  # PREFIX and SUFFIX include all existing content around the cursor, after quote removal.
  while [[ ${words[command_index]} == -C ]]; do (( command_index += 2 )); done
  if (( CURRENT > 2 )) && [[ ${words[command_index]} == commit && ${words[CURRENT-1]} == (-m|--message) ]]; then
    (( CURRENT == $#words )) || return 0
    [[ -z ${(Q)words[CURRENT]} && -z $PREFIX$SUFFIX ]] || return 0
    args=( "${(@Q)words[1,CURRENT]}" )
    args[-1]=''
    output=$(command git-autocomplete complete zsh -- "${args[@]}" 2>/dev/null)
    if (( $? == 0 )); then
      suggestions=( "${(@f)output}" )
      if [[ -n $output ]]; then
        if [[ -z $compstate[quote] ]]; then
          compadd -Q -V git-autocomplete-messages -d suggestions -- "${(@qqq)suggestions}"
        else
          compadd -V git-autocomplete-messages -- "${suggestions[@]}"
        fi
      fi
      return 0
    fi
  fi
  if [[ $_git_autocomplete_previous != _git_autocomplete_dispatch ]]; then
    "$_git_autocomplete_previous" "$@"
  fi
}
compdef _git_autocomplete_dispatch git
