/*!
Provides completions for ripgrep's CLI for the nushell shell.
*/

use crate::flags::{CompletionType, defs::FLAGS};

const PRELUDE: &'static str = "\
def \"nu-complete rg types\" [] {
  ^rg --type-list
  | lines
  | split column \": \"
  | rename value description
  | update value { str trim }
}

export extern \"rg\" [
  pattern?: string # The pattern to search for
  ...paths: path # The paths to search
";

const TEMPLATE_CHOICES: &'static str =
    "def \"nu-complete rg !LONG!\" [] { [!CHOICES!] }\n";

/// Generate completions for Nushell.
///
/// Reference: <https://www.nushell.sh/book/custom_completions.html>
pub(crate) fn generate() -> String {
    let mut helpers = String::new();
    let mut flags = String::new();
    for flag in FLAGS.iter() {
        let doc = flag.doc_short().replace('\n', " ");
        let short = match flag.name_short() {
            None => "".to_string(),
            Some(byte) => format!("(-{})", char::from(byte)),
        };
        let long = flag.name_long();

        let mut completion = format!("  --{long}{short}");
        if !flag.is_switch() {
            let value_type = match flag.completion_type() {
                CompletionType::Filename => "path".to_string(),
                CompletionType::Filetype => {
                    "string@\"nu-complete rg types\"".to_string()
                }
                CompletionType::Other if !flag.doc_choices().is_empty() => {
                    let choices = flag
                        .doc_choices()
                        .iter()
                        .map(|choice| format!("\"{choice}\""))
                        .collect::<Vec<String>>()
                        .join(", ");
                    helpers.push_str(
                        &TEMPLATE_CHOICES
                            .replace("!LONG!", long)
                            .replace("!CHOICES!", &choices),
                    );
                    format!("string@\"nu-complete rg {long}\"")
                }
                _ => "string".to_string(),
            };
            completion.push_str(&format!(": {value_type}"));
        }
        completion.push_str(&format!(" # {doc}"));
        completion.push('\n');
        flags.push_str(&completion);

        if let Some(negated) = flag.name_negated() {
            flags.push_str(&format!("  --{negated} # {doc}\n"));
        }
    }

    let mut out = String::new();
    out.push_str(&helpers);
    out.push_str(PRELUDE);
    out.push_str(&flags);
    out.push_str("]\n");
    out
}
