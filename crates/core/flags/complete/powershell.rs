/*!
Provides completions for ripgrep's CLI for PowerShell.
*/

use crate::flags::defs::FLAGS;

const TEMPLATE: &'static str = "
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'rg' -ScriptBlock {
  param($wordToComplete, $commandAst, $cursorPosition)
  $commandElements = $commandAst.CommandElements
  $command = @(
    'rg'
    for ($i = 1; $i -lt $commandElements.Count; $i++) {
        $element = $commandElements[$i]
        if ($element -isnot [StringConstantExpressionAst] -or
            $element.StringConstantType -ne [StringConstantType]::BareWord -or
            $element.Value.StartsWith('-')) {
            break
    }
    $element.Value
  }) -join ';'

  $completions = @(switch ($command) {
    'rg' {
!FLAGS!
    }
  })

  $completions.Where{ $_.CompletionText -like \"$wordToComplete*\" } |
    Sort-Object -Property ListItemText
}
";

const TEMPLATE_FLAG: &'static str = "[CompletionResult]::new('!DASH_NAME!', '!NAME!', [CompletionResultType]::ParameterName, '!DOC!')";

/// Generate completions for PowerShell.
///
/// Note that these completions are based on what was produced for ripgrep <=13
/// using Clap 2.x. Improvements on this are welcome.
pub(crate) fn generate() -> String {
    let mut flags = String::new();
    for (i, flag) in FLAGS.iter().enumerate() {
        let doc = flag.doc_short().replace("'", "''");

        let dash_name = format!("--{}", flag.name_long());
        let name = flag.name_long();
        if i > 0 {
            flags.push('\n');
        }
        flags.push_str("      ");
        flags.push_str(
            &TEMPLATE_FLAG
                .replace("!DASH_NAME!", &dash_name)
                .replace("!NAME!", &name)
                .replace("!DOC!", &doc),
        );

        if let Some(byte) = flag.name_short() {
            let dash_name = format!("-{}", char::from(byte));
            let name = char::from(byte).to_string();
            flags.push_str("\n      ");
            flags.push_str(
                &TEMPLATE_FLAG
                    .replace("!DASH_NAME!", &dash_name)
                    .replace("!NAME!", &name)
                    .replace("!DOC!", &doc),
            );
        }

        if let Some(negated) = flag.name_negated() {
            let dash_name = format!("--{negated}");
            flags.push_str("\n      ");
            flags.push_str(
                &TEMPLATE_FLAG
                    .replace("!DASH_NAME!", &dash_name)
                    .replace("!NAME!", &negated)
                    .replace("!DOC!", &doc),
            );
        }
    }
    TEMPLATE.trim_start().replace("!FLAGS!", &flags)
}
