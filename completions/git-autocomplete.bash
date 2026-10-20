# Source after Git's Bash completion. Prefer Bash 4.4+; empty quotes are not supported.
if [[ ${_git_autocomplete_loaded:-} == 1 ]]; then
  return 0
fi

_git_autocomplete_dispatch() {
  local previous current output line command_index=1
  local -a args
  previous=${COMP_WORDS[COMP_CWORD-1]}
  current=${COMP_WORDS[COMP_CWORD]}
  while [[ ${COMP_WORDS[command_index]} == -C ]]; do command_index=$((command_index+2)); done
  if [[ ${COMP_WORDS[command_index]} == commit && ( $previous == -m || $previous == --message ) ]]; then
    COMPREPLY=()
    if type compopt &>/dev/null; then
      compopt +o filenames +o default +o bashdefault 2>/dev/null || :
    fi
    # Preserve written messages and quote contexts; only complete a bare empty argument.
    [[ -z $current ]] || return 0
    line=${COMP_LINE:0:COMP_POINT}
    [[ $line == *[[:space:]] ]] || return 0
    [[ ${COMP_LINE:COMP_POINT} =~ ^[[:space:]]*$ ]] || return 0
    args=( "${COMP_WORDS[@]:0:COMP_CWORD}" '' )
    if output=$(command git-autocomplete complete bash -- "${args[@]}" 2>/dev/null); then
      if type compopt &>/dev/null; then
        compopt -o noquote 2>/dev/null || :
      fi
      while IFS= read -r line; do
        [[ -n $line ]] && COMPREPLY+=( "$line" )
      done <<< "$output"
      return 0
    fi
  fi
  if [[ -n ${_git_autocomplete_previous:-} ]]; then
    "$_git_autocomplete_previous" "$@"
  fi
}

_git_autocomplete_install() {
  local spec flag value
  local -a fields options
  # Load lazy Git completion before saving its function and options.
  if ! complete -p git &>/dev/null && type _completion_loader &>/dev/null; then
    _completion_loader git >/dev/null 2>&1 || :
  fi
  spec=$(complete -p git 2>/dev/null) || spec=''
  [[ -n $spec ]] || spec='complete -o default git'
  read -r -a fields <<< "$spec"
  options=()
  _git_autocomplete_previous=''
  local i=1
  while (( i < ${#fields[@]}-1 )); do
    flag=${fields[i]}
    value=${fields[i+1]}
    case $flag in
      -F) _git_autocomplete_previous=$value ;;
      -o)
        case $value in
          default|bashdefault|filenames)
            if ! type compopt &>/dev/null; then
              printf '%s\n' 'git-autocomplete: this completion setup requires Bash 4.4+ to preserve messages safely' >&2
              return 1
            fi
            ;;
        esac
        options+=( -o "$value" )
        ;;
      *) printf '%s\n' 'git-autocomplete: unsupported Git completion specification; load standard Git Bash completion first' >&2; return 1 ;;
    esac
    i=$((i+2))
  done
  # Function names are shell metadata, but never evaluate the printed completion specification.
  if [[ $_git_autocomplete_previous == \'*\' ]]; then
    _git_autocomplete_previous=${_git_autocomplete_previous:1:${#_git_autocomplete_previous}-2}
  fi
  complete "${options[@]}" -F _git_autocomplete_dispatch git
  _git_autocomplete_loaded=1
}
_git_autocomplete_install
