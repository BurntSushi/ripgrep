/*!
Provides completions for ripgrep's CLI for the bash shell.
*/

use crate::flags::defs::FLAGS;

const TEMPLATE_FULL: &'static str = "
_rg() {
  local i cur prev opts cmds
  COMPREPLY=()
  cur=\"${COMP_WORDS[COMP_CWORD]}\"
  prev=\"${COMP_WORDS[COMP_CWORD-1]}\"
  cmd=\"\"
  opts=\"\"

  for i in ${COMP_WORDS[@]}; do
    case \"${i}\" in
      rg)
        cmd=\"rg\"
        ;;
      *)
        ;;
    esac
  done

  case \"${cmd}\" in
    rg)
      opts=\"!OPTS!\"
      if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
        COMPREPLY=($(compgen -W \"${opts}\" -- \"${cur}\"))
        return 0
      fi
      case \"${prev}\" in
!CASES!
      esac
      COMPREPLY=($(compgen -W \"${opts}\" -- \"${cur}\"))
      return 0
      ;;
  esac
}

complete -F _rg -o bashdefault -o default rg
";

const TEMPLATE_CASE: &'static str = "
        !FLAG!)
          COMPREPLY=($(compgen -f \"${cur}\"))
          return 0
          ;;
";

const TEMPLATE_CASE_CHOICES: &'static str = "
        !FLAG!)
          COMPREPLY=($(compgen -W \"!CHOICES!\" -- \"${cur}\"))
          return 0
          ;;
";

/// Generate completions for Bash.
///
/// Note that these completions are based on what was produced for ripgrep <=13
/// using Clap 2.x. Improvements on this are welcome.
pub(crate) fn generate() -> String {
    let mut opts = String::new();
    for flag in FLAGS.iter() {
        opts.push_str("--");
        opts.push_str(flag.name_long());
        opts.push(' ');
        if let Some(short) = flag.name_short() {
            opts.push('-');
            opts.push(char::from(short));
            opts.push(' ');
        }
        if let Some(name) = flag.name_negated() {
            opts.push_str("--");
            opts.push_str(name);
            opts.push(' ');
        }
    }
    opts.push_str("<PATTERN> <PATH>...");

    let mut cases = String::new();
    for flag in FLAGS.iter() {
        let template = if !flag.doc_choices().is_empty() {
            let choices = flag.doc_choices().join(" ");
            TEMPLATE_CASE_CHOICES.trim_end().replace("!CHOICES!", &choices)
        } else {
            TEMPLATE_CASE.trim_end().to_string()
        };
        let name = format!("--{}", flag.name_long());
        cases.push_str(&template.replace("!FLAG!", &name));
        if let Some(short) = flag.name_short() {
            let name = format!("-{}", char::from(short));
            cases.push_str(&template.replace("!FLAG!", &name));
        }
        if let Some(negated) = flag.name_negated() {
            let name = format!("--{negated}");
            cases.push_str(&template.replace("!FLAG!", &name));
        }
    }

    TEMPLATE_FULL
        .replace("!OPTS!", &opts)
        .replace("!CASES!", &cases)
        .trim_start()
        .to_string()
}
