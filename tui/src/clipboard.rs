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

#[allow(dead_code)]
pub trait Clipboard {
    fn get_text(&mut self) -> Result<String, Error>;
    fn set_text(&mut self, text: String) -> Result<(), Error>;
}

pub struct ArboardClipboard {
    clipboard: arboard::Clipboard,
}

impl ArboardClipboard {
    pub fn new() -> Result<Self, Error> {
        let clipboard = arboard::Clipboard::new().map_err(Error::Access)?;
        Ok(Self { clipboard })
    }
}

impl Clipboard for ArboardClipboard {
    fn get_text(&mut self) -> Result<String, Error> {
        let text = self.clipboard.get_text().map_err(Error::GetText)?;
        Ok(text)
    }

    fn set_text(&mut self, text: String) -> Result<(), Error> {
        self.clipboard.set_text(text).map_err(Error::SetText)?;
        Ok(())
    }
}
