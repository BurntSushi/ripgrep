#compdef rg

##
# zsh completion function for ripgrep
#
# Run ci/test-complete after building to ensure that the options supported by
# this function stay in synch with the `rg` binary.
#
# For convenience, a completion reference guide is included at the bottom of
# this file.
#
# Originally based on code from the zsh-users project — see copyright notice
# below.

_rg() {
  local curcontext=$curcontext no='!' descr ret=1
  local -a context line state state_descr args tmp suf
  local -A opt_args

  # ripgrep has many options which negate the effect of a more common one — for
  # example, `--no-column` to negate `--column`, and `--messages` to negate
  # `--no-messages`. There are so many of these, and they're so infrequently
  # used, that some users will probably find it irritating if they're completed
  # indiscriminately, so let's not do that unless either the current prefix
  # matches one of those negation options or the user has the `complete-all`
  # style set. Note that this prefix check has to be updated manually to account
  # for all of the potential negation options listed below!
  if
    # We also want to list all of these options during testing
    [[ $_RG_COMPLETE_LIST_ARGS == (1|t*|y*) ]] ||
    # (--[imnp]* => --ignore*, --messages, --no-*, --pcre2-unicode)
    [[ $PREFIX$SUFFIX == --[imnp]* ]] ||
    zstyle -t ":completion:${curcontext}:" complete-all
  then
    no=
  fi

  # We make heavy use of argument groups here to prevent the option specs from
  # growing unwieldy. These aren't supported in zsh <5.4, though, so we'll strip
  # them out below if necessary. This makes the exclusions inaccurate on those
  # older versions, but oh well — it's not that big a deal
  args=(
!FLAGS!

    + operand # Operands
    '(--files --type-list file regexp)1: :_guard "^-*" pattern'
    '(--type-list)*: :_files'
  )

  # This is used with test-complete to verify that there are no options
  # listed in the help output that aren't also defined here
  [[ $_RG_COMPLETE_LIST_ARGS == (1|t*|y*) ]] && {
    print -rl - $args
    return 0
  }

  # Strip out argument groups where unsupported (see above)
  [[ $ZSH_VERSION == (4|5.<0-3>)(.*)# ]] &&
  args=( ${(@)args:#(#i)(+|[a-z0-9][a-z0-9_-]#|\([a-z0-9][a-z0-9_-]#\))} )

  _arguments -C -s -S : $args && ret=0

  case $state in
    colorspec)
      if [[ ${IPREFIX#--*=}$PREFIX == [^:]# ]]; then
        suf=( -qS: )
        tmp=(
          'column:specify coloring for column numbers'
          'line:specify coloring for line numbers'
          'match:specify coloring for match text'
          'highlight:specify coloring for matching lines'
          'path:specify coloring for file names'
        )
        descr='color/style type'
      elif [[ ${IPREFIX#--*=}$PREFIX == (column|line|match|highlight|path):[^:]# ]]; then
        suf=( -qS: )
        tmp=(
          'none:clear color/style for type'
          'bg:specify background color'
          'fg:specify foreground color'
          'style:specify text style'
        )
        descr='color/style attribute'
      elif [[ ${IPREFIX#--*=}$PREFIX == [^:]##:(bg|fg):[^:]# ]]; then
        tmp=( black blue green red cyan magenta yellow white )
        descr='color name or r,g,b'
      elif [[ ${IPREFIX#--*=}$PREFIX == [^:]##:style:[^:]# ]]; then
        tmp=( {,no}bold {,no}intense {,no}underline )
        descr='style name'
      else
        _message -e colorspec 'no more arguments'
      fi

      (( $#tmp )) && {
        compset -P '*:'
        _describe -t colorspec $descr tmp $suf && ret=0
      }
      ;;

    typespec)
      if compset -P '[^:]##:include:'; then
        _sequence -s , _rg_types && ret=0
      # @todo This bit in particular could be better, but it's a little
      # complex, and attempting to solve it seems to run us up against a crash
      # bug — zsh # 40362
      elif compset -P '[^:]##:'; then
        _message 'glob or include directive' && ret=1
      elif [[ ! -prefix *:* ]]; then
        _rg_types -qS : && ret=0
      fi
      ;;
  esac

  return ret
}

# Complete encodings
(( $+functions[_rg_encodings] )) ||
_rg_encodings() {
  local -a expl
  local -aU _encodings

  _encodings=(
!ENCODINGS!
  )

  _wanted encodings expl encoding compadd -a "$@" - _encodings
}

# Complete file types
(( $+functions[_rg_types] )) ||
_rg_types() {
  local -a expl
  local -aU _types

  _types=( ${(@)${(f)"$( _call_program types $words[1] --type-list )"}//:[[:space:]]##/:} )

  if zstyle -t ":completion:${curcontext}:types" extra-verbose; then
    _describe -t types 'file type' _types
  else
    _wanted types expl 'file type' compadd "$@" - ${(@)_types%%:*}
  fi
}

# Complete hyperlink format-string aliases
(( $+functions[_rg_hyperlink_format_aliases] )) ||
_rg_hyperlink_format_aliases() {
  _describe -t format-aliases 'hyperlink format alias' '(
!HYPERLINK_ALIASES!
  )'
}

# Complete custom hyperlink format strings
(( $+functions[_rg_hyperlink_format_strings] )) ||
_rg_hyperlink_format_strings() {
  local op='{' ed='}'
  local -a pfx sfx rmv

  compquote op ed

  sfx=( -S $ed )
  rmv=( -r ${(q)ed[1]} )

  compset -S "$op*"
  compset -S "$ed*" && sfx=( -S '' )
  compset -P "*$ed"
  compset -p ${#PREFIX%$op*}
  compset -P $op || pfx=( -P $op )

  WSL_DISTRO_NAME=${WSL_DISTRO_NAME:-\$WSL_DISTRO_NAME} \
  _describe -t format-variables 'hyperlink format variable' '(
    path:"absolute path to file containing match (required)"
    host:"system host name or output of --hostname-bin executable"
    line:"line number of match"
    column:"column of match (requires {line})"
    wslprefix:"\"wsl$/$WSL_DISTRO_NAME\" (for WSL share)"
  )' "${(@)pfx}" "${(@)sfx}" "${(@)rmv}"
}

# Complete hyperlink formats
(( $+functions[_rg_hyperlink_formats] )) ||
_rg_hyperlink_formats() {
  _alternative \
    'format-string-aliases: :_rg_hyperlink_format_aliases' \
    'format-strings: :_rg_hyperlink_format_strings'
}

# Don't run the completion function when being sourced by itself.
#
# See https://github.com/BurntSushi/ripgrep/issues/2956
# See https://github.com/BurntSushi/ripgrep/pull/2957
if [[ $funcstack[1] == _rg ]] || (( ! $+functions[compdef] )); then
  _rg "$@"
else
  compdef _rg rg
fi

################################################################################
# ZSH COMPLETION REFERENCE
#
# For the convenience of developers who aren't especially familiar with zsh
# completion functions, a brief reference guide follows. This is in no way
# comprehensive; it covers just enough of the basic structure, syntax, and
# conventions to help someone make simple changes like adding new options. For
# more complete documentation regarding zsh completion functions, please see the
# following:
#
# * http://zsh.sourceforge.net/Doc/Release/Completion-System.html
# * https://github.com/zsh-users/zsh/blob/master/Etc/completion-style-guide
#
# OVERVIEW
#
# Most zsh completion functions are defined in terms of `_arguments`, which is a
# shell function that takes a series of argument specifications. The specs for
# `rg` are stored in an array, which is common for more complex functions; the
# elements of the array are passed to `_arguments` on invocation.
#
# ARGUMENT-SPECIFICATION SYNTAX
#
# The following is a contrived example of the argument specs for a simple tool:
#
#   '(: * -)'{-h,--help}'[display help information]'
#   '(-q -v --quiet --verbose)'{-q,--quiet}'[decrease output verbosity]'
#   '!(-q -v --quiet --verbose)--silent'
#   '(-q -v --quiet --verbose)'{-v,--verbose}'[increase output verbosity]'
#   '--color=[specify when to use colors]:when:(always never auto)'
#   '*:example file:_files'
#
# Although there may appear to be six specs here, there are actually nine; we
# use brace expansion to combine specs for options that go by multiple names,
# like `-q` and `--quiet`. This is customary, and ties in with the fact that zsh
# merges completion possibilities together when they have the same description.
#
# The first line defines the option `-h`/`--help`. With most tools, it isn't
# useful to complete anything after `--help` because it effectively overrides
# all others; the `(: * -)` at the beginning of the spec tells zsh not to
# complete any other operands (`:` and `*`) or options (`-`) after this one has
# been used. The `[...]` at the end associates a description with `-h`/`--help`;
# as mentioned, zsh will see the identical descriptions and merge these options
# together when offering completion possibilities.
#
# The next line defines `-q`/`--quiet`. Here we don't want to suppress further
# completions entirely, but we don't want to offer `-q` if `--quiet` has been
# given (since they do the same thing), nor do we want to offer `-v` (since it
# doesn't make sense to be quiet and verbose at the same time). We don't need to
# tell zsh not to offer `--quiet` a second time, since that's the default
# behaviour, but since this line expands to two specs describing `-q` *and*
# `--quiet` we do need to explicitly list all of them here.
#
# The next line defines a hidden option `--silent` — maybe it's a deprecated
# synonym for `--quiet`. The leading `!` indicates that zsh shouldn't offer this
# option during completion. The benefit of providing a spec for an option that
# shouldn't be completed is that, if someone *does* use it, we can correctly
# suppress completion of other options afterwards.
#
# The next line defines `-v`/`--verbose`; this works just like `-q`/`--quiet`.
#
# The next line defines `--color`. In this example, `--color` doesn't have a
# corresponding short option, so we don't need to use brace expansion. Further,
# there are no other options it's exclusive with (just itself), so we don't need
# to define those at the beginning. However, it does take a mandatory argument.
# The `=` at the end of `--color=` indicates that the argument may appear either
# like `--color always` or like `--color=always`; this is how most GNU-style
# command-line tools work. The corresponding short option would normally use `+`
# — for example, `-c+` would allow either `-c always` or `-calways`. For this
# option, the arguments are known ahead of time, so we can simply list them in
# parentheses at the end (`when` is used as the description for the argument).
#
# The last line defines an operand (a non-option argument). In this example, the
# operand can be used any number of times (the leading `*`), and it should be a
# file path, so we tell zsh to call the `_files` function to complete it. The
# `example file` in the middle is the description to use for this operand; we
# could use a space instead to accept the default provided by `_files`.
#
# GROUPING ARGUMENT SPECIFICATIONS
#
# Newer versions of zsh support grouping argument specs together. All specs
# following a `+` and then a group name are considered to be members of the
# named group. Grouping is useful mostly for organisational purposes; it makes
# the relationship between different options more obvious, and makes it easier
# to specify exclusions.
#
# We could rewrite our example above using grouping as follows:
#
#   '(: * -)'{-h,--help}'[display help information]'
#   '--color=[specify when to use colors]:when:(always never auto)'
#   '*:example file:_files'
#   + '(verbosity)'
#   {-q,--quiet}'[decrease output verbosity]'
#   '!--silent'
#   {-v,--verbose}'[increase output verbosity]'
#
# Here we take advantage of a useful feature of spec grouping — when the group
# name is surrounded by parentheses, as in `(verbosity)`, it tells zsh that all
# of the options in that group are exclusive with each other. As a result, we
# don't need to manually list out the exclusions at the beginning of each
# option.
#
# Groups can also be referred to by name in other argument specs; for example:
#
#   '(xyz)--aaa' '*: :_files'
#   + xyz --xxx --yyy --zzz
#
# Here we use the group name `xyz` to tell zsh that `--xxx`, `--yyy`, and
# `--zzz` are not to be completed after `--aaa`. This makes the exclusion list
# much more compact and reusable.
#
# CONVENTIONS
#
# zsh completion functions generally adhere to the following conventions:
#
# * Use two spaces for indentation
# * Combine specs for options with different names using brace expansion
# * In combined specs, list the short option first (as in `{-a,--text}`)
# * Use `+` or `=` as described above for options that take arguments
# * Provide a description for all options, option-arguments, and operands
# * Capitalise/punctuate argument descriptions as phrases, not complete
#   sentences — 'display help information', never 'Display help information.'
#   (but still capitalise acronyms and proper names)
# * Write argument descriptions as verb phrases — 'display x', 'enable y',
#   'use z'
# * Word descriptions to make it clear when an option expects an argument;
#   usually this is done with the word 'specify', as in 'specify x' or
#   'use specified x')
# * Write argument descriptions as tersely as possible — for example, articles
#   like 'a' and 'the' should be omitted unless it would be confusing
#
# Other conventions currently used by this function:
#
# * Order argument specs alphabetically by group name, then option name
# * Group options that are directly related, mutually exclusive, or frequently
#   referenced by other argument specs
# * Use only characters in the set [a-z0-9_-] in group names
# * Order exclusion lists as follows: short options, long options, groups
# * Use American English in descriptions
# * Use 'don't' in descriptions instead of 'do not'
# * Word descriptions for related options as similarly as possible. For example,
#   `--foo[enable foo]` and `--no-foo[disable foo]`, or `--foo[use foo]` and
#   `--no-foo[don't use foo]`
# * Word descriptions to make it clear when an option only makes sense with
#   another option, usually by adding '(with -x)' to the end
# * Don't quote strings or variables unnecessarily. When quotes are required,
#   prefer single-quotes to double-quotes
# * Prefix option specs with `$no` when the option serves only to negate the
#   behaviour of another option that must be provided explicitly by the user.
#   This prevents rarely used options from cluttering up the completion menu
################################################################################

# ------------------------------------------------------------------------------
# Copyright (c) 2011 Github zsh-users - http://github.com/zsh-users
# All rights reserved.
#
# Redistribution and use in source and binary forms, with or without
# modification, are permitted provided that the following conditions are met:
#     * Redistributions of source code must retain the above copyright
#       notice, this list of conditions and the following disclaimer.
#     * Redistributions in binary form must reproduce the above copyright
#       notice, this list of conditions and the following disclaimer in the
#       documentation and/or other materials provided with the distribution.
#     * Neither the name of the zsh-users nor the
#       names of its contributors may be used to endorse or promote products
#       derived from this software without specific prior written permission.
#
# THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND
# ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
# WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
# DISCLAIMED. IN NO EVENT SHALL ZSH-USERS BE LIABLE FOR ANY
# DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES
# (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
# LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND
# ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
# (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
# SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
# ------------------------------------------------------------------------------
# Description
# -----------
#
#  Completion script for ripgrep
#
# ------------------------------------------------------------------------------
# Authors
# -------
#
#  * arcizan <ghostrevery@gmail.com>
#  * MaskRay <i@maskray.me>
#
# ------------------------------------------------------------------------------

# Local Variables:
# mode: shell-script
# coding: utf-8-unix
# indent-tabs-mode: nil
# sh-indentation: 2
# sh-basic-offset: 2
# End:
# vim: ft=zsh sw=2 ts=2 et
