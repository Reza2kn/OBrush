//! Bounded little-endian primitives for the private retained-state packet.
pub(crate) struct Writer {
    bytes: Vec<u8>,
    limit: usize,
}

impl Writer {
    pub(crate) fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            limit,
        }
    }
    pub(crate) fn raw(&mut self, value: &[u8]) -> Result<(), String> {
        let next = self
            .bytes
            .len()
            .checked_add(value.len())
            .ok_or("Source warm packet size overflow")?;
        if next > self.limit {
            return Err("Source warm packet exceeds byte limit".into());
        }
        if next > self.bytes.capacity() {
            let capacity = self
                .bytes
                .capacity()
                .saturating_mul(2)
                .max(next)
                .min(self.limit);
            self.bytes
                .try_reserve_exact(capacity - self.bytes.len())
                .map_err(|_| "Cannot reserve Source warm packet")?;
        }
        self.bytes.extend_from_slice(value);
        Ok(())
    }
    pub(crate) fn u8(&mut self, value: u8) -> Result<(), String> {
        self.raw(&[value])
    }
    pub(crate) fn u32(&mut self, value: u32) -> Result<(), String> {
        self.raw(&value.to_le_bytes())
    }
    pub(crate) fn u64(&mut self, value: u64) -> Result<(), String> {
        self.raw(&value.to_le_bytes())
    }
    pub(crate) fn usize(&mut self, value: usize) -> Result<(), String> {
        self.u32(u32::try_from(value).map_err(|_| "Source warm count exceeds u32")?)
    }
    pub(crate) fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

pub(crate) struct Reader<'a> {
    remaining: &'a [u8],
}
impl<'a> Reader<'a> {
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        Self { remaining: bytes }
    }
    pub(crate) fn raw(&mut self, count: usize) -> Result<&'a [u8], String> {
        let (head, tail) = self
            .remaining
            .split_at_checked(count)
            .ok_or("Truncated Source warm packet")?;
        self.remaining = tail;
        Ok(head)
    }
    pub(crate) fn u8(&mut self) -> Result<u8, String> {
        Ok(self.raw(1)?[0])
    }
    pub(crate) fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.raw(4)?.try_into().unwrap()))
    }
    pub(crate) fn u64(&mut self) -> Result<u64, String> {
        Ok(u64::from_le_bytes(self.raw(8)?.try_into().unwrap()))
    }
    pub(crate) fn usize(&mut self) -> Result<usize, String> {
        Ok(self.u32()? as usize)
    }
    pub(crate) fn done(&self) -> bool {
        self.remaining.is_empty()
    }
    pub(crate) fn remaining_len(&self) -> usize {
        self.remaining.len()
    }
}
