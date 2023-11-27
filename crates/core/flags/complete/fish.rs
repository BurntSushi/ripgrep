/*!
Provides completions for ripgrep's CLI for the fish shell.
*/

use crate::flags::defs::FLAGS;

const TEMPLATE: &'static str =
    "complete -c rg -n '__fish_use_subcommand' !SHORT! !LONG! !DOC!\n";
const TEMPLATE_CHOICES: &'static str =
    "complete -c rg -n '__fish_use_subcommand' !SHORT! !LONG! !DOC! -r -f -a '!CHOICES!'\n";

/// Generate completions for Fish.
///
/// Note that these completions are based on what was produced for ripgrep <=13
/// using Clap 2.x. Improvements on this are welcome.
pub(crate) fn generate() -> String {
    let mut out = String::new();
    for flag in FLAGS.iter() {
        let short = match flag.name_short() {
            None => "".to_string(),
            Some(byte) => format!("-s {}", char::from(byte)),
        };
        let long = format!("-l '{}'", flag.name_long().replace("'", "\\'"));
        let doc = format!("-d '{}'", flag.doc_short().replace("'", "\\'"));
        let template = if flag.doc_choices().is_empty() {
            TEMPLATE.to_string()
        } else {
            TEMPLATE_CHOICES
                .replace("!CHOICES!", &flag.doc_choices().join(" "))
        };
        out.push_str(
            &template
                .replace("!SHORT!", &short)
                .replace("!LONG!", &long)
                .replace("!DOC!", &doc),
        );
        if let Some(negated) = flag.name_negated() {
            let long = format!("-l '{}'", negated.replace("'", "\\'"));
            out.push_str(
                &TEMPLATE
                    .replace("!SHORT!", "")
                    .replace("!LONG!", &long)
                    .replace("!DOC!", &doc),
            );
        }
    }
    out
}
