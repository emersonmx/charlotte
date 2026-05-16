pub use confirm_quit_modal::{ConfirmQuitModal, Message as ConfirmQuitModalMessage};
pub use error_modal::{ErrorModal, Message as ErrorModalMessage};
pub use shortcuts_modal::{Message as ShortcutsModalMessage, ShortcutsModal};
pub use waiting_modal::WaitingModal;

mod confirm_quit_modal;
mod error_modal;
mod shortcuts_modal;
mod waiting_modal;
