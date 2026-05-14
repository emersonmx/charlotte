use arboard::Clipboard as ArboardClipboard;
use thiserror::Error;

#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to access clipboard: {0}")]
    Access(#[source] arboard::Error),
    #[error("Failed to set text to clipboard: {0}")]
    SetText(#[source] arboard::Error),
    #[error("Failed to get text from clipboard: {0}")]
    GetText(#[source] arboard::Error),
}

pub struct Clipboard {
    clipboard: ArboardClipboard,
}

#[allow(dead_code)]
impl Clipboard {
    pub fn new() -> Result<Self, Error> {
        Ok(Self {
            clipboard: ArboardClipboard::new().map_err(Error::Access)?,
        })
    }

    pub fn get_text(&mut self) -> Result<String, Error> {
        let text = self.clipboard.get_text().map_err(Error::GetText)?;
        Ok(text)
    }

    pub fn set_text(&mut self, text: String) -> Result<(), Error> {
        self.clipboard.set_text(text).map_err(Error::SetText)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};

    #[fixture]
    fn clipboard() -> Clipboard {
        Clipboard::new().expect("Failed to create clipboard")
    }

    #[rstest]
    fn create_clipboard() {
        let clipboard = Clipboard::new();
        assert!(
            clipboard.is_ok(),
            "Failed to create clipboard: {:?}",
            clipboard.err()
        );
    }
}
