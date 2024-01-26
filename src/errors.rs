use std::error::Error;

#[derive(Debug)]
pub struct MultipleErrors {
    errors: Vec<anyhow::Error>,
}

impl MultipleErrors {
    pub fn new(errors: Vec<anyhow::Error>) -> Self {
        Self {
            errors
        }
    }
}

impl Error for MultipleErrors {}

impl std::fmt::Display for MultipleErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Multiple errors:")?;
        for error in &self.errors {
            writeln!(f, "- {}", error)?;
        }
        Ok(())
    }
}