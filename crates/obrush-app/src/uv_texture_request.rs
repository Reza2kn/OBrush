//! Host ownership for a global viewport asset, separate from core edit history.
#[derive(Default, Debug)]
pub(crate) struct UvTextureRequests {
    last_issued: u32,
    pending: Option<u32>,
}

impl UvTextureRequests {
    pub fn begin(&mut self) -> Result<u32, String> {
        self.pending = None;
        let id = self
            .last_issued
            .checked_add(1)
            .ok_or("Viewport texture request IDs exhausted; reload before opening another file")?;
        self.last_issued = id;
        self.pending = Some(id);
        Ok(id)
    }
    pub fn is_current(&self, id: u32) -> bool {
        self.pending == Some(id)
    }
    pub fn cancel(&mut self, id: u32) -> bool {
        if !self.is_current(id) {
            return false;
        }
        self.pending = None;
        true
    }
    pub fn invalidate(&mut self) {
        self.pending = None;
    }
    pub fn pending(&self) -> Option<u32> {
        self.pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn newer_picker_owns_the_asset_and_old_cancel_cannot_cancel_it() {
        let mut owner = UvTextureRequests::default();
        let first = owner.begin().unwrap();
        let second = owner.begin().unwrap();
        assert!(second > first);
        assert!(!owner.is_current(first));
        assert!(!owner.cancel(first));
        assert_eq!(owner.pending(), Some(second));
        assert!(owner.cancel(second));
        assert!(!owner.is_current(second));
        assert!(!owner.cancel(second));
    }
    #[test]
    fn reset_invalidates_old_deliveries_without_reusing_ids() {
        let mut owner = UvTextureRequests::default();
        let first = owner.begin().unwrap();
        owner.invalidate();
        assert!(!owner.is_current(first));
        let second = owner.begin().unwrap();
        assert!(second > first);
        assert!(!owner.is_current(first));
    }
    #[test]
    fn request_id_overflow_rejects_without_reuse_or_a_pending_owner() {
        let mut owner = UvTextureRequests {
            last_issued: u32::MAX,
            pending: Some(u32::MAX),
        };
        assert!(owner.begin().is_err());
        assert_eq!(owner.pending(), None);
        assert!(!owner.is_current(0));
    }
}
