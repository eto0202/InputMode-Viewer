#[derive(PartialEq, Clone, Copy, Debug)]
pub enum ShowState {
    Hidden,
    Visible,
}

impl ShowState {
    pub fn update(&mut self, displayed: bool) -> bool {
        match (displayed, *self) {
            (false, _) => {
                *self = ShowState::Hidden;
                false
            }
            (true, ShowState::Hidden) => {
                *self = ShowState::Visible;
                true
            }
            (true, ShowState::Visible) => false,
        }
    }
}
