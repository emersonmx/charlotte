use crossterm::event::{Event, KeyCode, KeyEvent};

pub fn is_quit_pressed(event: &Event) -> bool {
    if let Event::Key(KeyEvent { code, .. }) = event {
        matches!(code, KeyCode::Char('q'))
    } else {
        false
    }
}

pub fn is_accept_pressed(event: &Event) -> bool {
    if let Event::Key(KeyEvent { code, .. }) = event {
        matches!(code, KeyCode::Enter)
    } else {
        false
    }
}

pub fn is_back_pressed(event: &Event) -> bool {
    if let Event::Key(KeyEvent { code, .. }) = event {
        matches!(code, KeyCode::Backspace | KeyCode::Esc)
    } else {
        false
    }
}

pub fn is_copy_pressed(event: &Event) -> bool {
    if let Event::Key(KeyEvent { code, .. }) = event {
        matches!(code, KeyCode::Char('y'))
    } else {
        false
    }
}

pub fn get_section_number(event: &Event) -> Option<usize> {
    match event {
        Event::Key(KeyEvent { code, .. }) => {
            if let KeyCode::Char(c) = code {
                c.to_digit(10).map(|d| d as usize)
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn is_up_pressed(event: &Event) -> bool {
    if let Event::Key(KeyEvent { code, .. }) = event {
        matches!(code, KeyCode::Up | KeyCode::Char('k'))
    } else {
        false
    }
}

pub fn is_down_pressed(event: &Event) -> bool {
    if let Event::Key(KeyEvent { code, .. }) = event {
        matches!(code, KeyCode::Down | KeyCode::Char('j'))
    } else {
        false
    }
}

pub fn is_left_pressed(event: &Event) -> bool {
    if let Event::Key(KeyEvent { code, .. }) = event {
        matches!(code, KeyCode::Left | KeyCode::Char('h'))
    } else {
        false
    }
}

pub fn is_right_pressed(event: &Event) -> bool {
    if let Event::Key(KeyEvent { code, .. }) = event {
        matches!(code, KeyCode::Right | KeyCode::Char('l'))
    } else {
        false
    }
}

pub fn is_previous_tab_pressed(event: &Event) -> bool {
    if let Event::Key(KeyEvent { code, .. }) = event {
        matches!(code, KeyCode::BackTab | KeyCode::Char('['))
    } else {
        false
    }
}

pub fn is_next_tab_pressed(event: &Event) -> bool {
    if let Event::Key(KeyEvent { code, .. }) = event {
        matches!(code, KeyCode::Tab | KeyCode::Char(']'))
    } else {
        false
    }
}

pub fn is_page_up_pressed(event: &Event) -> bool {
    if let Event::Key(KeyEvent { code, .. }) = event {
        matches!(code, KeyCode::PageUp)
    } else {
        false
    }
}

pub fn is_page_down_pressed(event: &Event) -> bool {
    if let Event::Key(KeyEvent { code, .. }) = event {
        matches!(code, KeyCode::PageDown)
    } else {
        false
    }
}

pub fn is_home_pressed(event: &Event) -> bool {
    if let Event::Key(KeyEvent { code, .. }) = event {
        matches!(code, KeyCode::Home)
    } else {
        false
    }
}

pub fn is_end_pressed(event: &Event) -> bool {
    if let Event::Key(KeyEvent { code, .. }) = event {
        matches!(code, KeyCode::End)
    } else {
        false
    }
}

pub fn is_any_key_pressed(event: &Event) -> bool {
    matches!(event, Event::Key(_))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{
        Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    };
    use rstest::rstest;

    #[rstest]
    fn test_is_quit_pressed() {
        let quit_event = Event::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        let non_quit_event = Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));

        assert!(is_quit_pressed(&quit_event));
        assert!(!is_quit_pressed(&non_quit_event));
    }

    #[rstest]
    fn test_is_accept_pressed() {
        let accept_event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        let non_accept_event = Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));

        assert!(is_accept_pressed(&accept_event));
        assert!(!is_accept_pressed(&non_accept_event));
    }

    #[rstest]
    fn test_is_back_pressed() {
        let backspace_event = Event::Key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
        let esc_event = Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        let non_back_event = Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));

        assert!(is_back_pressed(&backspace_event));
        assert!(is_back_pressed(&esc_event));
        assert!(!is_back_pressed(&non_back_event));
    }

    #[rstest]
    fn test_is_copy_pressed() {
        let copy_event = Event::Key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
        let non_copy_event = Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));

        assert!(is_copy_pressed(&copy_event));
        assert!(!is_copy_pressed(&non_copy_event));
    }

    #[rstest]
    fn test_get_section_number() {
        let section_1_event = Event::Key(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE));
        let section_5_event = Event::Key(KeyEvent::new(KeyCode::Char('5'), KeyModifiers::NONE));
        let non_section_event = Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));

        assert_eq!(get_section_number(&section_1_event), Some(1));
        assert_eq!(get_section_number(&section_5_event), Some(5));
        assert_eq!(get_section_number(&non_section_event), None);
    }

    #[rstest]
    fn test_is_up_pressed() {
        let up_event = Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        let k_event = Event::Key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
        let non_up_event = Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));

        assert!(is_up_pressed(&up_event));
        assert!(is_up_pressed(&k_event));
        assert!(!is_up_pressed(&non_up_event));
    }

    #[rstest]
    fn test_is_down_pressed() {
        let down_event = Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        let j_event = Event::Key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
        let non_down_event = Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));

        assert!(is_down_pressed(&down_event));
        assert!(is_down_pressed(&j_event));
        assert!(!is_down_pressed(&non_down_event));
    }

    #[rstest]
    fn test_is_left_pressed() {
        let left_event = Event::Key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
        let h_event = Event::Key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));
        let non_left_event = Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));

        assert!(is_left_pressed(&left_event));
        assert!(is_left_pressed(&h_event));
        assert!(!is_left_pressed(&non_left_event));
    }

    #[rstest]
    fn test_is_right_pressed() {
        let right_event = Event::Key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
        let l_event = Event::Key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE));
        let non_right_event = Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));

        assert!(is_right_pressed(&right_event));
        assert!(is_right_pressed(&l_event));
        assert!(!is_right_pressed(&non_right_event));
    }

    #[rstest]
    fn test_is_prev_tab_pressed() {
        let backtab_event = Event::Key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE));
        let left_bracket_event = Event::Key(KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE));
        let non_prev_tab_event = Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));

        assert!(is_previous_tab_pressed(&backtab_event));
        assert!(is_previous_tab_pressed(&left_bracket_event));
        assert!(!is_previous_tab_pressed(&non_prev_tab_event));
    }

    #[rstest]
    fn test_is_next_tab_pressed() {
        let tab_event = Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        let right_bracket_event = Event::Key(KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE));
        let non_next_tab_event = Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));

        assert!(is_next_tab_pressed(&tab_event));
        assert!(is_next_tab_pressed(&right_bracket_event));
        assert!(!is_next_tab_pressed(&non_next_tab_event));
    }

    #[rstest]
    fn test_is_page_up_pressed() {
        let page_up_event = Event::Key(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE));
        let non_page_up_event = Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));

        assert!(is_page_up_pressed(&page_up_event));
        assert!(!is_page_up_pressed(&non_page_up_event));
    }

    #[rstest]
    fn test_is_page_down_pressed() {
        let page_down_event = Event::Key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE));
        let non_page_down_event = Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));

        assert!(is_page_down_pressed(&page_down_event));
        assert!(!is_page_down_pressed(&non_page_down_event));
    }

    #[rstest]
    fn test_is_home_pressed() {
        let home_event = Event::Key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE));
        let non_home_event = Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));

        assert!(is_home_pressed(&home_event));
        assert!(!is_home_pressed(&non_home_event));
    }

    #[rstest]
    fn test_is_end_pressed() {
        let end_event = Event::Key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE));
        let non_end_event = Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));

        assert!(is_end_pressed(&end_event));
        assert!(!is_end_pressed(&non_end_event));
    }

    #[rstest]
    fn test_is_any_key_pressed() {
        let key_event = Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        let non_key_event = Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        });

        assert!(is_any_key_pressed(&key_event));
        assert!(!is_any_key_pressed(&non_key_event));
    }
}
