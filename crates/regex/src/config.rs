use grep_matcher::{ByteSet, LineTerminator};
use regex::bytes::{Regex, RegexBuilder};
use regex_syntax::ast::{self, Ast};
use regex_syntax::hir::{self, Hir};

use crate::ast::AstAnalysis;
use crate::crlf::crlfify;
use crate::error::Error;
use crate::literal::LiteralSets;
use crate::multi::alternation_literals;
use crate::non_matching::non_matching_bytes;
use crate::strip::strip_from_match;

/// Config represents the configuration of a regex matcher in this crate.
/// The configuration is itself a rough combination of the knobs found in
/// the `regex` crate itself, along with additional `grep-matcher` specific
/// options.
///
/// The configuration can be used to build a "configured" HIR expression. A
/// configured HIR expression is an HIR expression that is aware of the
/// configuration which generated it, and provides transformation on that HIR
/// such that the configuration is preserved.
#[derive(Clone, Debug)]
pub struct Config {
    pub case_insensitive: bool,
    pub case_smart: bool,
    pub multi_line: bool,
    pub dot_matches_new_line: bool,
    pub swap_greed: bool,
    pub ignore_whitespace: bool,
    pub unicode: bool,
    pub octal: bool,
    pub size_limit: usize,
    pub dfa_size_limit: usize,
    pub nest_limit: u32,
    pub line_terminator: Option<LineTerminator>,
    pub crlf: bool,
    pub word: bool,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            case_insensitive: false,
            case_smart: false,
            multi_line: false,
            dot_matches_new_line: false,
            swap_greed: false,
            ignore_whitespace: false,
            unicode: true,
            octal: false,
            // These size limits are much bigger than what's in the regex
            // crate.
            size_limit: 100 * (1 << 20),
            dfa_size_limit: 1000 * (1 << 20),
            nest_limit: 250,
            line_terminator: None,
            crlf: false,
            word: false,
        }
    }
}

impl Config {
    /// Parse the given pattern and returned its HIR expression along with
    /// the current configuration.
    ///
    /// If there was a problem parsing the given expression then an error
    /// is returned.
    pub fn hir(&self, pattern: &str) -> Result<ConfiguredHIR, Error> {
        let ast = self.ast(pattern)?;
        let analysis = self.analysis(&ast)?;
        let expr = hir::translate::TranslatorBuilder::new()
            .allow_invalid_utf8(true)
            .case_insensitive(self.is_case_insensitive(&analysis))
            .multi_line(self.multi_line)
            .dot_matches_new_line(self.dot_matches_new_line)
            .swap_greed(self.swap_greed)
            .unicode(self.unicode)
            .build()
            .translate(pattern, &ast)
            .map_err(Error::regex)?;
        let expr = match self.line_terminator {
            None => expr,
            Some(line_term) => strip_from_match(expr, line_term)?,
        };
        Ok(ConfiguredHIR {
            original: pattern.to_string(),
            config: self.clone(),
            analysis,
            // If CRLF mode is enabled, replace `$` with `(?:\r?$)`.
            expr: if self.crlf { crlfify(expr) } else { expr },
        })
    }

    /// Accounting for the `smart_case` config knob, return true if and only if
    /// this pattern should be matched case insensitively.
    fn is_case_insensitive(&self, analysis: &AstAnalysis) -> bool {
        if self.case_insensitive {
            return true;
        }
        if !self.case_smart {
            return false;
        }
        analysis.any_literal() && !analysis.any_uppercase()
    }

    /// Returns true if and only if this config is simple enough such that
    /// if the pattern is a simple alternation of literals, then it can be
    /// constructed via a plain Aho-Corasick automaton.
    ///
    /// Note that it is OK to return true even when settings like `multi_line`
    /// are enabled, since if multi-line can impact the match semantics of a
    /// regex, then it is by definition not a simple alternation of literals.
    pub fn can_plain_aho_corasick(&self) -> bool {
        !self.word && !self.case_insensitive && !self.case_smart
    }

    /// Perform analysis on the AST of this pattern.
    ///
    /// This returns an error if the given pattern failed to parse.
    fn analysis(&self, ast: &Ast) -> Result<AstAnalysis, Error> {
        Ok(AstAnalysis::from_ast(ast))
    }

    /// Parse the given pattern into its abstract syntax.
    ///
    /// This returns an error if the given pattern failed to parse.
    fn ast(&self, pattern: &str) -> Result<Ast, Error> {
        ast::parse::ParserBuilder::new()
            .nest_limit(self.nest_limit)
            .octal(self.octal)
            .ignore_whitespace(self.ignore_whitespace)
            .build()
            .parse(pattern)
            .map_err(Error::regex)
    }
}

/// A "configured" HIR expression, which is aware of the configuration which
/// produced this HIR.
///
/// Since the configuration is tracked, values with this type can be
/// transformed into other HIR expressions (or regular expressions) in a way
/// that preserves the configuration. For example, the `fast_line_regex`
/// method will apply literal extraction to the inner HIR and use that to build
/// a new regex that matches the extracted literals in a way that is
/// consistent with the configuration that produced this HIR. For example, the
/// size limits set on the configured HIR will be propagated out to any
/// subsequently constructed HIR or regular expression.
#[derive(Clone, Debug)]
pub struct ConfiguredHIR {
    original: String,
    config: Config,
    analysis: AstAnalysis,
    expr: Hir,
}

impl ConfiguredHIR {
    /// Return the configuration for this HIR expression.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Compute the set of non-matching bytes for this HIR expression.
    pub fn non_matching_bytes(&self) -> ByteSet {
        non_matching_bytes(&self.expr)
    }

    /// Returns true if and only if this regex needs to have its match offsets
    /// tweaked because of CRLF support. Specifically, this occurs when the
    /// CRLF hack is enabled and the regex is line anchored at the end. In
    /// this case, matches that end with a `\r` have the `\r` stripped.
    pub fn needs_crlf_stripped(&self) -> bool {
        self.config.crlf && self.expr.is_line_anchored_end()
    }

    /// Builds a regular expression from this HIR expression.
    pub fn regex(&self) -> Result<Regex, Error> {
        self.pattern_to_regex(&self.expr.to_string())
    }

    /// If this HIR corresponds to an alternation of literals with no
    /// capturing groups, then this returns those literals.
    pub fn alternation_literals(&self) -> Option<Vec<Vec<u8>>> {
        if !self.config.can_plain_aho_corasick() {
            return None;
        }
        alternation_literals(&self.expr)
    }

    /// Applies the given function to the concrete syntax of this HIR and then
    /// generates a new HIR based on the result of the function in a way that
    /// preserves the configuration.
    ///
    /// For example, this can be used to wrap a user provided regular
    /// expression with additional semantics. e.g., See the `WordMatcher`.
    pub fn with_pattern<F: FnMut(&str) -> String>(
        &self,
        mut f: F,
    ) -> Result<ConfiguredHIR, Error> {
        self.pattern_to_hir(&f(&self.expr.to_string()))
    }

    /// If the current configuration has a line terminator set and if useful
    /// literals could be extracted, then a regular expression matching those
    /// literals is returned. If no line terminator is set, then `None` is
    /// returned.
    ///
    /// If compiling the resulting regular expression failed, then an error
    /// is returned.
    ///
    /// This method only returns something when a line terminator is set
    /// because matches from this regex are generally candidates that must be
    /// confirmed before reporting a match. When performing a line oriented
    /// search, confirmation is easy: just extend the candidate match to its
    /// respective line boundaries and then re-search that line for a full
    /// match. This only works when the line terminator is set because the line
    /// terminator setting guarantees that the regex itself can never match
    /// through the line terminator byte.
    pub fn fast_line_regex(&self) -> Result<Option<Regex>, Error> {
        if self.config.line_terminator.is_none() {
            return Ok(None);
        }
        match LiteralSets::new(&self.expr).one_regex(self.config.word) {
            None => Ok(None),
            Some(pattern) => self.pattern_to_regex(&pattern).map(Some),
        }
    }

    /// Create a regex from the given pattern using this HIR's configuration.
    fn pattern_to_regex(&self, pattern: &str) -> Result<Regex, Error> {
        // The settings we explicitly set here are intentionally a subset
        // of the settings we have. The key point here is that our HIR
        // expression is computed with the settings in mind, such that setting
        // them here could actually lead to unintended behavior. For example,
        // consider the pattern `(?U)a+`. This will get folded into the HIR
        // as a non-greedy repetition operator which will in turn get printed
        // to the concrete syntax as `a+?`, which is correct. But if we
        // set the `swap_greed` option again, then we'll wind up with `(?U)a+?`
        // which is equal to `a+` which is not the same as what we were given.
        //
        // We also don't need to apply `case_insensitive` since this gets
        // folded into the HIR and would just cause us to do redundant work.
        //
        // Finally, we don't need to set `ignore_whitespace` since the concrete
        // syntax emitted by the HIR printer never needs it.
        //
        // We set the rest of the options. Some of them are important, such as
        // the size limit, and some of them are necessary to preserve the
        // intention of the original pattern. For example, the Unicode flag
        // will impact how the WordMatcher functions, namely, whether its
        // word boundaries are Unicode aware or not.
        RegexBuilder::new(&pattern)
            .nest_limit(self.config.nest_limit)
            .octal(self.config.octal)
            .multi_line(self.config.multi_line)
            .dot_matches_new_line(self.config.dot_matches_new_line)
            .unicode(self.config.unicode)
            .size_limit(self.config.size_limit)
            .dfa_size_limit(self.config.dfa_size_limit)
            .build()
            .map_err(Error::regex)
    }

    /// Create an HIR expression from the given pattern using this HIR's
    /// configuration.
    fn pattern_to_hir(&self, pattern: &str) -> Result<ConfiguredHIR, Error> {
        // See `pattern_to_regex` comment for explanation of why we only set
        // a subset of knobs here. e.g., `swap_greed` is explicitly left out.
        let expr = ::regex_syntax::ParserBuilder::new()
            .nest_limit(self.config.nest_limit)
            .octal(self.config.octal)
            .allow_invalid_utf8(true)
            .multi_line(self.config.multi_line)
            .dot_matches_new_line(self.config.dot_matches_new_line)
            .unicode(self.config.unicode)
            .build()
            .parse(pattern)
            .map_err(Error::regex)?;
        Ok(ConfiguredHIR {
            original: self.original.clone(),
            config: self.config.clone(),
            analysis: self.analysis.clone(),
            expr,
        })
    }
}
