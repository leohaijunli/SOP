//! Parse errors.
//!
//! A parse error means the file cannot be interpreted at all, as opposed to a
//! diagnostic, which means it was interpreted and found wanting.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("file does not start with a '---' front matter delimiter")]
    MissingFrontMatter { line: usize },

    #[error("front matter is not closed by a '---' delimiter")]
    UnclosedFrontMatter,

    #[error("front matter is not valid YAML: {source}")]
    FrontMatterYaml {
        #[from]
        source: serde_norway::Error,
    },

    #[error("front matter must be a YAML mapping")]
    FrontMatterNotMapping,

    #[error("unclosed code fence")]
    UnclosedFence { line: usize },

    #[error("'{info}' block is not a YAML mapping")]
    BlockNotMapping { info: String, line: usize },

    #[error("'{info}' block is not valid YAML: {source}")]
    BlockYaml {
        info: String,
        line: usize,
        source: serde_norway::Error,
    },

    #[error("{message}")]
    Invalid { message: String, line: usize },
}

impl ParseError {
    /// The line the error is attributed to, when it can be attributed to one.
    pub fn line(&self) -> Option<usize> {
        match self {
            ParseError::MissingFrontMatter { line } => Some(*line),
            ParseError::UnclosedFence { line } | ParseError::BlockNotMapping { line, .. } => {
                Some(*line)
            }
            ParseError::Invalid { line, .. } => Some(*line),
            ParseError::BlockYaml { line, .. } => Some(*line),
            ParseError::UnclosedFrontMatter
            | ParseError::FrontMatterYaml { .. }
            | ParseError::FrontMatterNotMapping => None,
        }
    }
}
