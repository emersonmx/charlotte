use crossterm::event::{Event, KeyCode, KeyEvent};

#[derive(Debug, Clone, PartialEq)]
pub enum Input {
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
    Home,
    End,
    Section(usize),
    Accept,
    Back,
    Copy,
    PreviousTab,
    NextTab,
    Quit,
    UnmappedKey,
}

pub fn map_event_to_input(event: &Event) -> Option<Input> {
    match event {
        Event::Key(KeyEvent { code, .. }) => match code {
            KeyCode::Up | KeyCode::Char('k') => Some(Input::Up),
            KeyCode::Down | KeyCode::Char('j') => Some(Input::Down),
            KeyCode::Left | KeyCode::Char('h') => Some(Input::Left),
            KeyCode::Right | KeyCode::Char('l') => Some(Input::Right),
            KeyCode::PageUp => Some(Input::PageUp),
            KeyCode::PageDown => Some(Input::PageDown),
            KeyCode::Home => Some(Input::Home),
            KeyCode::End => Some(Input::End),
            KeyCode::Char(c) if let Some(digit) = c.to_digit(10) => {
                Some(Input::Section(digit as usize))
            }
            KeyCode::Enter => Some(Input::Accept),
            KeyCode::Backspace | KeyCode::Esc => Some(Input::Back),
            KeyCode::Char('y') => Some(Input::Copy),
            KeyCode::BackTab | KeyCode::Char('[') => Some(Input::PreviousTab),
            KeyCode::Tab | KeyCode::Char(']') => Some(Input::NextTab),
            KeyCode::Char('q') => Some(Input::Quit),
            _ => Some(Input::UnmappedKey),
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{
        Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    };
    use rstest::rstest;

    #[rstest]
    fn test_map_event_to_input() {
        let up_event = Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        let down_event = Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        let left_event = Event::Key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
        let right_event = Event::Key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
        let page_up_event = Event::Key(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE));
        let page_down_event = Event::Key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE));
        let home_event = Event::Key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE));
        let end_event = Event::Key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE));
        let section_event = Event::Key(KeyEvent::new(KeyCode::Char('3'), KeyModifiers::NONE));
        let accept_event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        let back_event = Event::Key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
        let copy_event = Event::Key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
        let prev_tab_event = Event::Key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE));
        let next_tab_event = Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        let quit_event = Event::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        let unmapped_key_event = Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        let non_key_event = Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        });

        assert_eq!(map_event_to_input(&up_event), Some(Input::Up));
        assert_eq!(map_event_to_input(&down_event), Some(Input::Down));
        assert_eq!(map_event_to_input(&left_event), Some(Input::Left));
        assert_eq!(map_event_to_input(&right_event), Some(Input::Right));
        assert_eq!(map_event_to_input(&page_up_event), Some(Input::PageUp));
        assert_eq!(map_event_to_input(&page_down_event), Some(Input::PageDown));
        assert_eq!(map_event_to_input(&home_event), Some(Input::Home));
        assert_eq!(map_event_to_input(&end_event), Some(Input::End));
        assert_eq!(map_event_to_input(&section_event), Some(Input::Section(3)));
        assert_eq!(map_event_to_input(&accept_event), Some(Input::Accept));
        assert_eq!(map_event_to_input(&back_event), Some(Input::Back));
        assert_eq!(map_event_to_input(&copy_event), Some(Input::Copy));
        assert_eq!(
            map_event_to_input(&prev_tab_event),
            Some(Input::PreviousTab)
        );
        assert_eq!(map_event_to_input(&next_tab_event), Some(Input::NextTab));
        assert_eq!(map_event_to_input(&quit_event), Some(Input::Quit));
        assert_eq!(
            map_event_to_input(&unmapped_key_event),
            Some(Input::UnmappedKey)
        );
        assert_eq!(map_event_to_input(&non_key_event), None);
    }
}
