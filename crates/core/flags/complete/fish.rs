/*!
Provides completions for ripgrep's CLI for the fish shell.
*/

use crate::flags::{defs::FLAGS, CompletionType};

const TEMPLATE: &'static str = "complete -c rg !SHORT! -l !LONG! -d '!DOC!'";
const TEMPLATE_NEGATED: &'static str =
    "complete -c rg -l !NEGATED! -n '__fish_contains_opt !SHORT! !LONG!' -d '!DOC!'\n";

/// Generate completions for Fish.
pub(crate) fn generate() -> String {
    let mut out = String::new();
    for flag in FLAGS.iter() {
        let short = match flag.name_short() {
            None => "".to_string(),
            Some(byte) => format!("-s {}", char::from(byte)),
        };
        let long = flag.name_long();
        let doc = flag.doc_short().replace("'", "\\'");
        let mut completion = TEMPLATE
            .replace("!SHORT!", &short)
            .replace("!LONG!", &long)
            .replace("!DOC!", &doc);

        match flag.completion_type() {
            CompletionType::Filename => {
                completion.push_str(" -r -F");
            }
            CompletionType::Executable => {
                completion.push_str(" -r -f -a '(__fish_complete_command)'");
            }
            CompletionType::Filetype => {
                completion.push_str(
                    " -r -f -a '(rg --type-list | string replace : \\t)'",
                );
            }
            CompletionType::Encoding => {
                completion.push_str(" -r -f -a '");
                completion.push_str(super::ENCODINGS);
                completion.push_str("'");
            }
            CompletionType::Other if !flag.doc_choices().is_empty() => {
                completion.push_str(" -r -f -a '");
                completion.push_str(&flag.doc_choices().join(" "));
                completion.push_str("'");
            }
            CompletionType::Other if !flag.is_switch() => {
                completion.push_str(" -r -f");
            }
            CompletionType::Other => (),
        }

        completion.push('\n');
        out.push_str(&completion);

        if let Some(negated) = flag.name_negated() {
            out.push_str(
                &TEMPLATE_NEGATED
                    .replace("!NEGATED!", &negated)
                    .replace("!SHORT!", &short)
                    .replace("!LONG!", &long)
                    .replace("!DOC!", &doc),
            );
        }
    }
    out
}
