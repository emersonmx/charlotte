use crossterm::event::{Event, KeyCode, KeyEvent};

#[derive(Debug, Clone, PartialEq)]
pub enum Input {
    Quit,
    Accept,
    Back,
    Copy,
    Section(usize),
    Up,
    Down,
    Left,
    Right,
    PreviousTab,
    NextTab,
    PageUp,
    PageDown,
    Home,
    End,
    AnyKey,
}

pub fn map_event_to_input(event: &Event) -> Option<Input> {
    if is_quit_pressed(event) {
        Some(Input::Quit)
    } else if is_accept_pressed(event) {
        Some(Input::Accept)
    } else if is_back_pressed(event) {
        Some(Input::Back)
    } else if is_copy_pressed(event) {
        Some(Input::Copy)
    } else if let Some(section_number) = get_section_number(event) {
        Some(Input::Section(section_number))
    } else if is_up_pressed(event) {
        Some(Input::Up)
    } else if is_down_pressed(event) {
        Some(Input::Down)
    } else if is_left_pressed(event) {
        Some(Input::Left)
    } else if is_right_pressed(event) {
        Some(Input::Right)
    } else if is_previous_tab_pressed(event) {
        Some(Input::PreviousTab)
    } else if is_next_tab_pressed(event) {
        Some(Input::NextTab)
    } else if is_page_up_pressed(event) {
        Some(Input::PageUp)
    } else if is_page_down_pressed(event) {
        Some(Input::PageDown)
    } else if is_home_pressed(event) {
        Some(Input::Home)
    } else if is_end_pressed(event) {
        Some(Input::End)
    } else if is_any_key_pressed(event) {
        Some(Input::AnyKey)
    } else {
        None
    }
}

fn is_quit_pressed(event: &Event) -> bool {
    if let Event::Key(KeyEvent { code, .. }) = event {
        matches!(code, KeyCode::Char('q'))
    } else {
        false
    }
}

fn is_accept_pressed(event: &Event) -> bool {
    if let Event::Key(KeyEvent { code, .. }) = event {
        matches!(code, KeyCode::Enter)
    } else {
        false
    }
}

fn is_back_pressed(event: &Event) -> bool {
    if let Event::Key(KeyEvent { code, .. }) = event {
        matches!(code, KeyCode::Backspace | KeyCode::Esc)
    } else {
        false
    }
}

fn is_copy_pressed(event: &Event) -> bool {
    if let Event::Key(KeyEvent { code, .. }) = event {
        matches!(code, KeyCode::Char('y'))
    } else {
        false
    }
}

fn get_section_number(event: &Event) -> Option<usize> {
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

fn is_up_pressed(event: &Event) -> bool {
    if let Event::Key(KeyEvent { code, .. }) = event {
        matches!(code, KeyCode::Up | KeyCode::Char('k'))
    } else {
        false
    }
}

fn is_down_pressed(event: &Event) -> bool {
    if let Event::Key(KeyEvent { code, .. }) = event {
        matches!(code, KeyCode::Down | KeyCode::Char('j'))
    } else {
        false
    }
}

fn is_left_pressed(event: &Event) -> bool {
    if let Event::Key(KeyEvent { code, .. }) = event {
        matches!(code, KeyCode::Left | KeyCode::Char('h'))
    } else {
        false
    }
}

fn is_right_pressed(event: &Event) -> bool {
    if let Event::Key(KeyEvent { code, .. }) = event {
        matches!(code, KeyCode::Right | KeyCode::Char('l'))
    } else {
        false
    }
}

fn is_previous_tab_pressed(event: &Event) -> bool {
    if let Event::Key(KeyEvent { code, .. }) = event {
        matches!(code, KeyCode::BackTab | KeyCode::Char('['))
    } else {
        false
    }
}

fn is_next_tab_pressed(event: &Event) -> bool {
    if let Event::Key(KeyEvent { code, .. }) = event {
        matches!(code, KeyCode::Tab | KeyCode::Char(']'))
    } else {
        false
    }
}

fn is_page_up_pressed(event: &Event) -> bool {
    if let Event::Key(KeyEvent { code, .. }) = event {
        matches!(code, KeyCode::PageUp)
    } else {
        false
    }
}

fn is_page_down_pressed(event: &Event) -> bool {
    if let Event::Key(KeyEvent { code, .. }) = event {
        matches!(code, KeyCode::PageDown)
    } else {
        false
    }
}

fn is_home_pressed(event: &Event) -> bool {
    if let Event::Key(KeyEvent { code, .. }) = event {
        matches!(code, KeyCode::Home)
    } else {
        false
    }
}

fn is_end_pressed(event: &Event) -> bool {
    if let Event::Key(KeyEvent { code, .. }) = event {
        matches!(code, KeyCode::End)
    } else {
        false
    }
}

fn is_any_key_pressed(event: &Event) -> bool {
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
    fn test_map_event_to_input() {
        let quit_event = Event::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        let accept_event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        let back_event = Event::Key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
        let copy_event = Event::Key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
        let section_event = Event::Key(KeyEvent::new(KeyCode::Char('3'), KeyModifiers::NONE));
        let up_event = Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        let down_event = Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        let left_event = Event::Key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
        let right_event = Event::Key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
        let prev_tab_event = Event::Key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE));
        let next_tab_event = Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        let page_up_event = Event::Key(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE));
        let page_down_event = Event::Key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE));
        let home_event = Event::Key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE));
        let end_event = Event::Key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE));
        let any_key_event = Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        let non_key_event = Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        });

        assert_eq!(map_event_to_input(&quit_event), Some(Input::Quit));
        assert_eq!(map_event_to_input(&accept_event), Some(Input::Accept));
        assert_eq!(map_event_to_input(&back_event), Some(Input::Back));
        assert_eq!(map_event_to_input(&copy_event), Some(Input::Copy));
        assert_eq!(map_event_to_input(&section_event), Some(Input::Section(3)));
        assert_eq!(map_event_to_input(&up_event), Some(Input::Up));
        assert_eq!(map_event_to_input(&down_event), Some(Input::Down));
        assert_eq!(map_event_to_input(&left_event), Some(Input::Left));
        assert_eq!(map_event_to_input(&right_event), Some(Input::Right));
        assert_eq!(
            map_event_to_input(&prev_tab_event),
            Some(Input::PreviousTab)
        );
        assert_eq!(map_event_to_input(&next_tab_event), Some(Input::NextTab));
        assert_eq!(map_event_to_input(&page_up_event), Some(Input::PageUp));
        assert_eq!(map_event_to_input(&page_down_event), Some(Input::PageDown));
        assert_eq!(map_event_to_input(&home_event), Some(Input::Home));
        assert_eq!(map_event_to_input(&end_event), Some(Input::End));
        assert_eq!(map_event_to_input(&any_key_event), Some(Input::AnyKey));
        assert_eq!(map_event_to_input(&non_key_event), None);
    }

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
