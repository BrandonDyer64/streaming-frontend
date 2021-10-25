use glfw::{Action, Key, WindowEvent};

use crate::state::AsyncState;

pub async fn handle_event(event: WindowEvent, state: AsyncState) -> bool {
    let mut state = state.write().await;
    match event {
        WindowEvent::Close | WindowEvent::Key(Key::Escape, _, Action::Release, _) => return false,
        WindowEvent::Key(Key::Right, _, Action::Press | Action::Repeat, _) => {
            let new = state.selected_card.0.saturating_add(1);
            if new < state.rows[state.selected_card.1].cards.len() {
                state.selected_card.0 = new;
            }
        }
        WindowEvent::Key(Key::Left, _, Action::Press | Action::Repeat, _) => {
            state.selected_card.0 = state.selected_card.0.saturating_sub(1);
        }
        WindowEvent::Key(Key::Up, _, Action::Press, _) => {
            let new = state.selected_card.1.saturating_sub(1);
            if new < state.rows.len() {
                let scroll_old = state.rows[state.selected_card.1].scroll.round() as isize;
                let scroll_new = state.rows[new].scroll.round() as isize;
                let mut new_scrl = state.selected_card.0 as isize;
                new_scrl += scroll_new - scroll_old;
                state.selected_card.0 = new_scrl as usize;
                state.selected_card.1 = new;
            }
        }
        WindowEvent::Key(Key::Down, _, Action::Press, _) => {
            let new = state.selected_card.1.saturating_add(1);
            if new < state.rows.len() {
                let scroll_old = state.rows[state.selected_card.1].scroll.round() as isize;
                let scroll_new = state.rows[new].scroll.round() as isize;
                let mut new_scrl = state.selected_card.0 as isize;
                new_scrl += scroll_new - scroll_old;
                state.selected_card.0 = new_scrl as usize;
                state.selected_card.1 = new;
            }
        }
        _ => (),
    }
    true
}
