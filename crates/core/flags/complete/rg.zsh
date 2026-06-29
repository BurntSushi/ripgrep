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
  local no='!'
  local -a args

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
    # --[imnpru]* => --ignore*, --messages, --no-*, --pcre2-unicode,
    #                --require-git, --unicode
    [[ $PREFIX$SUFFIX == --[imnpru]* ]] ||
    zstyle -t ":completion:${curcontext}:" complete-all
  then
    no=
  fi

  args=(
    + '(exclusive)'
    '(: * -)'{-h,--help}'[display help information]'
    '(: * -)'{-V,--version}'[display version information]'
    '(: * -)'--pcre2-version'[print the version of PCRE2 used by ripgrep, if available]'

    + '(buffered)'
    '--line-buffered[force line buffering]'
    $no"--no-line-buffered[don't force line buffering]"
    '--block-buffered[force block buffering]'
    $no"--no-block-buffered[don't force block buffering]"

    + '(byte-offset)'
    {-b,--byte-offset}'[show 0-based byte offset for each matching line]'
    $no"--no-byte-offset[don't show byte offset for each matching line]"

    + '(case)'
    {-i,--ignore-case}'[search case-insensitively]'
    {-s,--case-sensitive}'[search case-sensitively]'
    {-S,--smart-case}'[search case-insensitively if pattern is all lowercase]'

    + '(context-a)'
    '(context-c)'{-A+,--after-context=}'[specify lines to show after each match]: :\
      _numbers -d0 -l0 "lines of context after match"
    '

    + '(context-b)'
    '(context-c)'{-B+,--before-context=}'[specify lines to show before each match]: :\
      _numbers -d0 -l0 "lines of context before match"
    '

    + '(context-c)'
    '(context-a context-b)'{-C+,--context=}'[specify lines to show before and after each match]: :\
      _numbers -d0 -l0 "lines of context before/after match"
    '

    + '(column)'
    '--column[show column numbers for matches]'
    $no"--no-column[don't show column numbers for matches]"

    + '(context-separator)'
    '--context-separator=[specify string used to separate non-continuous context lines in output]:separator'
    $no"--no-context-separator[don't print context separators]"

    + '(count)'
    {-c,--count}'[only show count of matching lines for each file]'
    '--count-matches[only show count of individual matches for each file]'
    '--include-zero[include files with zero matches in summary]'
    $no"--no-include-zero[don't include files with zero matches in summary]"

    + '(encoding)'
    {-E+,--encoding=}'[specify text encoding of files to search]: :_rg_encodings'
    $no'--no-encoding[use default text encoding]'

    + '(engine)'
    '--engine=[specify regex engine to use]:regex engine:((
      default\:"default engine"
      pcre2\:"identical to --pcre2"
      auto\:"identical to --auto-hybrid-regex"
    ))'

    + file
    '(1)*'{-f+,--file=}'[specify file containing patterns to search for]: :_files'

    + '(file-match)'
    '(stats)'{-l,--files-with-matches}'[only show names of files with matches]'
    '(stats)--files-without-match[only show names of files without matches]'

    + '(file-name)'
    {-H,--with-filename}'[show file name for matches]'
    {-I,--no-filename}"[don't show file name for matches]"

    + '(file-system)'
    "--one-file-system[don't descend into directories on other file systems]"
    $no'--no-one-file-system[descend into directories on other file systems]'

    + '(fixed)'
    {-F,--fixed-strings}'[treat pattern as literal string instead of regular expression]'
    $no"--no-fixed-strings[don't treat pattern as literal string]"

    + '(follow)'
    {-L,--follow}'[follow symlinks]'
    $no"--no-follow[don't follow symlinks]"

    + glob
    '*'{-g+,--glob=}'[include/exclude files matching specified glob]:glob pattern:_files'
    '*--iglob=[include/exclude files matching specified case-insensitive glob]:glob pattern:_files'

    + '(glob-case-insensitive)'
    '--glob-case-insensitive[treat -g/--glob patterns case insensitively]'
    $no'--no-glob-case-insensitive[treat -g/--glob patterns case sensitively]'

    + '(heading)'
    '(pretty-vimgrep)--heading[show matches grouped by file name]'
    "(pretty-vimgrep)--no-heading[don't show matches grouped by file name]"

    + '(hidden)'
    {-.,--hidden}'[search hidden files and directories]'
    $no"--no-hidden[don't search hidden files and directories]"

    + '(hybrid)'
    '--auto-hybrid-regex[DEPRECATED: dynamically use PCRE2 if necessary]'
    $no"--no-auto-hybrid-regex[DEPRECATED: don't dynamically use PCRE2 if necessary]"

    + '(ignore)'
    "(--no-ignore-global --no-ignore-parent --no-ignore-vcs --no-ignore-dot)--no-ignore[don't respect ignore files]"
    $no'(--ignore-global --ignore-parent --ignore-vcs --ignore-dot)--ignore[respect ignore files]'

    + '(ignore-file-case-insensitive)'
    '--ignore-file-case-insensitive[process ignore files case insensitively]'
    $no'--no-ignore-file-case-insensitive[process ignore files case sensitively]'

    + '(ignore-exclude)'
    "--no-ignore-exclude[don't respect local exclude (ignore) files]"
    $no'--ignore-exclude[respect local exclude (ignore) files]'

    + '(ignore-global)'
    "--no-ignore-global[don't respect global ignore files]"
    $no'--ignore-global[respect global ignore files]'

    + '(ignore-parent)'
    "--no-ignore-parent[don't respect ignore files in parent directories]"
    $no'--ignore-parent[respect ignore files in parent directories]'

    + '(ignore-vcs)'
    "--no-ignore-vcs[don't respect version control ignore files]"
    $no'--ignore-vcs[respect version control ignore files]'

    + '(require-git)'
    "--no-require-git[don't require git repository to respect gitignore rules]"
    $no'--require-git[require git repository to respect gitignore rules]'

    + '(ignore-dot)'
    "--no-ignore-dot[don't respect .ignore files]"
    $no'--ignore-dot[respect .ignore files]'

    + '(ignore-files)'
    "--no-ignore-files[don't respect --ignore-file flags]"
    $no'--ignore-files[respect --ignore-file files]'

    + '(invert-match)'
    {-v,--invert-match}'[invert matching]'
    $no"--no-invert-match[don't invert matching]"

    + '(json)'
    '--json[output results in JSON Lines format]'
    $no"--no-json[don't output results in JSON Lines format]"

    + '(line-number)'
    {-n,--line-number}'[show line numbers for matches]'
    {-N,--no-line-number}"[don't show line numbers for matches]"

    + '(line-terminator)'
    '--crlf[use CRLF as line terminator]'
    $no"--no-crlf[don't use CRLF as line terminator]"
    '(text)--null-data[use NUL as line terminator]'

    + '(max-columns-preview)'
    '--max-columns-preview[show preview for long lines (with -M)]'
    $no"--no-max-columns-preview[don't show preview for long lines (with -M)]"

    + '(max-depth)'
    {-d,--max-depth,\!--maxdepth}'[specify max number of directories to descend into]: :\
      _numbers -dunlimited -l0 "max directories to descend into"
    '

    + '(messages)'
    '(--no-ignore-messages)--no-messages[suppress some error messages]'
    $no"--messages[don't suppress error messages affected by --no-messages]"

    + '(messages-ignore)'
    "--no-ignore-messages[don't show ignore-file parse error messages]"
    $no'--ignore-messages[show ignore-file parse error messages]'

    + '(mmap)'
    '--mmap[search using memory maps when possible]'
    "--no-mmap[don't search using memory maps]"

    + '(multiline)'
    {-U,--multiline}'[permit matching across multiple lines]'
    $no'(multiline-dotall)--no-multiline[restrict matches to at most one line each]'

    + '(multiline-dotall)'
    '(--no-multiline)--multiline-dotall[allow "." to match newline (with -U)]'
    $no"(--no-multiline)--no-multiline-dotall[don't allow \".\" to match newline (with -U)]"

    + '(only)'
    {-o,--only-matching}'[show only matching part of each line]'

    + '(passthru)'
    '(--vimgrep)--passthru[show both matching and non-matching lines]'
    '!(--vimgrep)--passthrough'

    + '(pcre2)'
    {-P,--pcre2}'[enable matching with PCRE2]'
    $no'(pcre2-unicode)--no-pcre2[disable matching with PCRE2]'

    + '(pcre2-unicode)'
    $no'(--no-pcre2 --no-pcre2-unicode)--pcre2-unicode[DEPRECATED: enable PCRE2 Unicode mode (with -P)]'
    '(--no-pcre2 --pcre2-unicode)--no-pcre2-unicode[DEPRECATED: disable PCRE2 Unicode mode (with -P)]'

    + '(pre)'
    '(-z --search-zip)--pre=[specify preprocessor utility]:preprocessor utility:_command_names -e'
    $no'--no-pre[disable preprocessor utility]'

    + '(pretty-vimgrep)'
    '(heading)'{-p,--pretty}'[alias for --color=always --heading -n]'
    '(heading passthru)--vimgrep[show results in vim-compatible format]'

    + regexp
    '(1 file)*'{-e+,--regexp=}'[specify search pattern]:pattern'

    + '(replace)'
    {-r+,--replace=}'[specify string used to replace matches]:replace string'

    + '(sort)'
    '(threads)--sort=[sort results in ascending order (disables parallelism)]:sort method:((
      none\:"no sorting"
      path\:"sort by file path"
      modified\:"sort by last modified time"
      accessed\:"sort by last accessed time"
      created\:"sort by creation time"
    ))'
    '(threads)--sortr=[sort results in descending order (disables parallelism)]:sort method:((
      none\:"no sorting"
      path\:"sort by file path"
      modified\:"sort by last modified time"
      accessed\:"sort by last accessed time"
      created\:"sort by creation time"
    ))'
    '(threads)--sort-files[DEPRECATED: sort results by file path (disables parallelism)]'
    $no"--no-sort-files[DEPRECATED: don't sort results]"

    + '(stats)'
    '(--files file-match)--stats[show search statistics]'
    $no"--no-stats[don't show search statistics]"

    + '(text)'
    {-a,--text}'[search binary files as if they were text]'
    "--binary[search binary files, don't print binary data]"
    $no"--no-binary[don't search binary files]"
    $no"(--null-data)--no-text[don't search binary files as if they were text]"

    + '(threads)'
    '(sort)'{-j+,--threads=}'[specify approximate number of threads to use]: :\
      _numbers -d0 -l0 "max threads"
    '

    + '(trim)'
    '--trim[trim any ASCII whitespace prefix from each line]'
    $no"--no-trim[don't trim ASCII whitespace prefix from each line]"

    + type
    '*'{-t+,--type=}'[only search files matching specified type]: :_rg_types'
    '*--type-add=[add new glob for specified file type]: :_rg_type_specs'
    '*--type-clear=[clear globs previously defined for specified file type]: :_rg_types'
    # This should actually be exclusive with everything but other type options
    '(: *)--type-list[show all supported file types and their associated globs]'
    '*'{-T+,--type-not=}"[don't search files matching specified file type]: :_rg_types"

    + '(word-line)'
    {-w,--word-regexp}'[only show matches surrounded by word boundaries]'
    {-x,--line-regexp}'[only show matches surrounded by line boundaries]'

    + '(unicode)'
    $no'--unicode[enable Unicode mode]'
    '--no-unicode[disable Unicode mode]'

    + '(zip)'
    '(--pre)'{-z,--search-zip}'[search in compressed files]'
    $no"--no-search-zip[don't search in compressed files]"

    + misc
    '--color=[specify when to use colors in output]:when to use colors:((
      never\:"never use colors"
      auto\:"use colors or not based on stdout, TERM, etc."
      always\:"always use colors"
      ansi\:"always use ANSI colors (even on Windows)"
    ))'
    '*--colors=[specify color and style settings]: :_rg_color_specs'
    '--debug[show debug messages]'
    '--field-context-separator[set string to delimit fields in context lines]'
    '--field-match-separator[set string to delimit fields in matching lines]'
    '--generate=[generate specified data (e.g. man page or completion script)]:data to generate:((
      man\:"man page"
      complete-bash\:"shell completions for bash"
      complete-zsh\:"shell completions for zsh"
      complete-fish\:"shell completions for fish"
      complete-powershell\:"shell completions for PowerShell"
    ))'
    '--hostname-bin=[specify executable for getting system hostname]:hostname executable:_command_names -e'
    '--hyperlink-format=[specify pattern for hyperlinks]: :_rg_hyperlink_formats'
    '--trace[show more verbose debug messages]'
    '--dfa-size-limit=[specify upper size limit of generated DFA]: :\
      _numbers -ubytes "max DFA size" K M G
    '
    "(1 stats)--files[show each file that would be searched (but don't search)]"
    '*--ignore-file=[specify additional ignore file]:ignore file:_files'
    '(-M --max-columns)'{-M+,--max-columns=}'[specify max length of lines to print]: :\
      _numbers -ubytes -d0 -l0 "max line length"
    '
    '(-m --max-count)'{-m+,--max-count=}'[specify max number of matches per file]: :\
      _numbers -dunlimited -l0 "max matches per file"
    '
    '--max-filesize=[specify size above which files should be ignored]: :\
      _numbers -ubytes -dunlimited -l0 "max file size" K M G
    '
    "--no-config[don't load configuration files]"
    '(-0 --null)'{-0,--null}'[print NUL byte after file names]'
    '--path-separator=[specify path separator to use when printing file names]:separator'
    '*--pre-glob=[include/exclude files matching specified glob for preprocessing (with --pre)]:glob pattern:_files'
    '(-q --quiet)'{-q,--quiet}'[suppress normal output]'
    '--regex-size-limit=[specify upper size limit of compiled regex]: :\
      _numbers -ubytes "max compiled regex size" K M G
    '
    '*'{-u,--unrestricted}'[reduce level of "smart" searching]'
    '--stop-on-nonmatch[stop on first non-matching line after a matching one]'

    + operand
    '(--files --type-list file regexp)1: :_guard "^-*" "search pattern"'
    '(--type-list)*:file or directory to search:_files'
  )

  # This is used with test-complete to verify that there are no options
  # listed in the help output that aren't also defined here
  [[ $_RG_COMPLETE_LIST_ARGS == (1|t*|y*) ]] && {
    print -rl - $args
    return 0
  }

  _arguments -s -S : $args
}

# Complete colour specs
(( $+functions[_rg_color_specs] )) ||
_rg_color_specs() {
  local tag=color-specs

  if compset -P '[^:]##:(bg|fg):'; then
    _describe -t $tag 'color name or r,g,b' '(
      black blue green red cyan magenta yellow white
    )'

  elif compset -P '[^:]##:style:'; then
    _describe -t $tag 'style name' '(
      {,no}bold {,no}intense {,no}underline
    )'

  elif compset -P '[^:]##:'; then
    _describe -t $tag 'color/style attribute' \
      '(
        bg:"specify background color"
        fg:"specify foreground color"
        style:"specify text style"
      )' -r: -S: \
      -- \
      '( none:"clear color/style for type" )'

  else
    _describe -t $tag 'color/style type' '(
      column:"specify coloring for column numbers"
      line:"specify coloring for line numbers"
      match:"specify coloring for match text"
      highlight:"specify coloring for matching lines"
      path:"specify coloring for file names"
    )' -r: -S:
  fi
}

# Complete type specs
(( $+functions[_rg_type_specs] )) ||
_rg_type_specs() {
  local ret=1
  local -a expl

  if compset -P '[^:]##:include:'; then
    _sequence -s , _rg_types && ret=0

  elif compset -P '[^:]##:'; then
    _describe -t type-specs-include 'include directive' '(
      include:"include other type definitions"
    )' -r: -S: && ret=0
    _wanted type-specs-glob expl 'glob pattern' _files && ret=0

  else
    _wanted type-specs-name expl 'file type (or new name)' \
      _rg_types -r: -S: && ret=0
  fi

  return ret
}

# Complete encodings
(( $+functions[_rg_encodings] )) ||
_rg_encodings() {
  local -aU encodings=(
!ENCODINGS!
  )
  encodings=( ${encodings//:/\\:} )
  _describe -t encodings encoding encodings "$@"
}

# Complete file types
(( $+functions[_rg_types] )) ||
_rg_types() {
  local -aU types=(
    # this assumes none of the patterns contain colons
    ${(@)${(f)"$( _call_program types $words[1] --type-list )"}//:[[:space:]]##/:}
  )

  zstyle -t ":completion:${curcontext}:types" extra-verbose ||
  types=( ${types%%:*} )

  _describe -t types 'file type' types "$@"
}

# Complete hyperlink format-string aliases
(( $+functions[_rg_hyperlink_format_aliases] )) ||
_rg_hyperlink_format_aliases() {
  _describe -t format-aliases 'hyperlink format alias' '(
!HYPERLINK_ALIASES!
  )' "$@"
}

# Complete custom hyperlink format strings
(( $+functions[_rg_hyperlink_format_strings] )) ||
_rg_hyperlink_format_strings() {
  local bs
  local -a pfx sfx rmv

  [[ -n $compstate[quote] ]] || bs=\\

  sfx=( -S "$bs}" )
  rmv=( -r '}\-' )

  compset -S "$bs{*"
  compset -S "$bs}*" && sfx=( -S '' )
  compset -P '*}'
  compset -P '[^{}]##'
  compset -P '*{' || pfx=( -P "$bs{" )

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
