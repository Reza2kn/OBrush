//! Contact pressure and a retained terminal sample across UI event boundaries.
#[derive(Debug)]
pub(crate) struct ContactPressure {
    current: f32,
    terminal: Option<f32>,
    contact: bool,
}

impl Default for ContactPressure {
    fn default() -> Self {
        Self {
            current: 1.0,
            terminal: None,
            contact: false,
        }
    }
}

impl ContactPressure {
    pub fn contact(&mut self, pressure: f32) {
        self.current = if pressure.is_finite() {
            pressure.clamp(0.0, 1.0)
        } else {
            1.0
        };
        self.contact = true;
    }
    pub fn release(&mut self) {
        self.terminal = Some(self.current);
        self.contact = false;
    }
    pub fn mouse(&mut self) {
        self.current = 1.0;
        self.contact = false;
    }
    pub fn current(&self) -> f32 {
        self.current
    }
    pub fn endpoint(&self) -> f32 {
        self.terminal.unwrap_or(self.current)
    }
    pub fn finish_release(&mut self) {
        if self.terminal.take().is_some() && !self.contact {
            self.current = 1.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pen_release_then_mouse_command_keeps_terminal_pressure() {
        let mut p = ContactPressure::default();
        p.contact(0.25);
        p.release();
        p.mouse();
        assert_eq!(p.current(), 1.0);
        assert_eq!(p.endpoint(), 0.25);
        p.finish_release();
        assert_eq!(p.endpoint(), 1.0);
    }
    #[test]
    fn zero_release_and_a_new_contact_are_independent() {
        let mut p = ContactPressure::default();
        p.contact(0.0);
        p.release();
        p.contact(0.8);
        assert_eq!(p.endpoint(), 0.0);
        p.finish_release();
        assert_eq!(p.current(), 0.8);
        p.contact(f32::NAN);
        assert_eq!(p.current(), 1.0);
    }
}
