//! Ephemeral exact transaction evidence, never a public/persisted format or hash.
//! The same human-readable Serialize projection is traversed. Number bits and
//! unambiguous type/length markers avoid decimal work for mesh f32 channels.
use serde::ser::{self, Serialize};
use std::{fmt, io};
#[derive(Debug)]
pub(super) enum Error {
    Io(io::Error),
    Message(String),
    Limit,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => e.fmt(f),
            Self::Message(e) => e.fmt(f),
            Self::Limit => f.write_str("Stored state encoding limit"),
        }
    }
}
impl std::error::Error for Error {}
impl ser::Error for Error {
    fn custom<T: fmt::Display>(message: T) -> Self {
        Self::Message(message.to_string())
    }
}
pub(super) fn to_writer<W: io::Write, T: Serialize + ?Sized>(
    writer: &mut W,
    value: &T,
) -> Result<(), Error> {
    value.serialize(&mut Encoder { writer, depth: 0 })
}
struct Encoder<'w, W> {
    writer: &'w mut W,
    depth: usize,
}
#[cfg(test)]
const COMMON_F32: [f32; 16] = [
    0., 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1., -0., 0.25, 0.75, 0.125, 0.375,
];
fn common_f32(bits: u32) -> Option<u8> {
    Some(match bits {
        0x00000000 => 0,
        0x3dcccccd => 1,
        0x3e4ccccd => 2,
        0x3e99999a => 3,
        0x3ecccccd => 4,
        0x3f000000 => 5,
        0x3f19999a => 6,
        0x3f333333 => 7,
        0x3f4ccccd => 8,
        0x3f666666 => 9,
        0x3f800000 => 10,
        0x80000000 => 11,
        0x3e800000 => 12,
        0x3f400000 => 13,
        0x3e000000 => 14,
        0x3ec00000 => 15,
        _ => return None,
    })
}
impl<W: io::Write> Encoder<'_, W> {
    fn bytes(&mut self, b: &[u8]) -> Result<(), Error> {
        self.writer.write_all(b).map_err(Error::Io)
    }
    fn tag(&mut self, t: u8) -> Result<(), Error> {
        self.bytes(&[t])
    }
    fn integer(&mut self, tag: u8, mut value: u128) -> Result<(), Error> {
        let mut b = [0u8; 20];
        b[0] = tag;
        let mut n = 1;
        loop {
            let part = (value & 127) as u8;
            value >>= 7;
            b[n] = part | if value == 0 { 0 } else { 128 };
            n += 1;
            if value == 0 {
                break;
            }
        }
        self.bytes(&b[..n])
    }
    fn size(&mut self, value: usize) -> Result<(), Error> {
        let mut v = u64::try_from(value).map_err(|_| Error::Limit)?;
        let mut b = [0u8; 10];
        let mut n = 0;
        loop {
            b[n] = (v & 127) as u8;
            v >>= 7;
            if v != 0 {
                b[n] |= 128;
            }
            n += 1;
            if v == 0 {
                break;
            }
        }
        self.bytes(&b[..n])
    }
    fn text(&mut self, value: &str) -> Result<(), Error> {
        self.size(value.len())?;
        self.bytes(value.as_bytes())
    }
    fn scalar(&mut self, tag: u8, bits: &[u8]) -> Result<(), Error> {
        let mut b = [0u8; 17];
        b[0] = tag;
        b[1..=bits.len()].copy_from_slice(bits);
        self.bytes(&b[..=bits.len()])
    }
    fn f32_bits(&mut self, value: f32, base: u8, common: u8) -> Result<(), Error> {
        let bits = value.to_bits();
        if let Some(i) = common_f32(bits) {
            return self.tag(common + i);
        }
        let b = bits.to_le_bytes();
        let skip = b.iter().take_while(|&&v| v == 0).count();
        self.scalar(base + skip as u8, &b[skip..])
    }
    fn f64_bits(&mut self, value: f64) -> Result<(), Error> {
        if value.is_nan() {
            return self.scalar(0xc0, &value.to_bits().to_le_bytes());
        }
        let narrow = value as f32;
        if (narrow as f64).to_bits() == value.to_bits() {
            return self.f32_bits(narrow, 0xb0, 0x90);
        }
        if value.is_finite() {
            // Only uncommon f64 channels need the old exact shortest decimal.
            // A fixed stack sink avoids an uncharged formatted String.
            struct Number {
                bytes: [u8; 32],
                len: usize,
            }
            impl io::Write for Number {
                fn write(&mut self, b: &[u8]) -> io::Result<usize> {
                    let end = self
                        .len
                        .checked_add(b.len())
                        .ok_or_else(|| io::Error::other("Number span"))?;
                    if end > 32 {
                        return Err(io::Error::other("Number span"));
                    }
                    self.bytes[self.len..end].copy_from_slice(b);
                    self.len = end;
                    Ok(b.len())
                }
                fn flush(&mut self) -> io::Result<()> {
                    Ok(())
                }
            }
            let mut number = Number {
                bytes: [0; 32],
                len: 0,
            };
            serde_json::to_writer(&mut number, &value)
                .map_err(|e| Error::Message(e.to_string()))?;
            if number.len < 8 {
                self.tag(0xd0 + number.len as u8)?;
                return self.bytes(&number.bytes[..number.len]);
            }
        }
        self.scalar(0xc0, &value.to_bits().to_le_bytes())
    }
    fn enter(&mut self) -> Result<(), Error> {
        if self.depth >= 64 {
            return Err(Error::Limit);
        }
        self.depth += 1;
        Ok(())
    }
    fn container(&mut self, object: bool, len: Option<usize>) -> Result<(), Error> {
        self.enter()?;
        let base = if object { 0x60 } else { 0x40 };
        match len {
            Some(n) if n < 15 => self.tag(base + n as u8),
            Some(n) => {
                self.tag(base + 15)?;
                self.size(n)
            }
            None => self.tag(if object { 0x70 } else { 0x50 }),
        }
    }
    fn nested<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        self.enter()?;
        let r = value.serialize(&mut *self);
        self.depth -= 1;
        r
    }
    fn end(&mut self, unknown: bool) -> Result<(), Error> {
        if unknown {
            self.tag(0xff)?;
        }
        self.depth -= 1;
        Ok(())
    }
}
struct Compound<'a, 'w, W> {
    encoder: &'a mut Encoder<'w, W>,
    expected: Option<usize>,
    count: usize,
    pending_key: bool,
    outer: bool,
}
impl<'a, 'w, W: io::Write> Compound<'a, 'w, W> {
    fn new(encoder: &'a mut Encoder<'w, W>, expected: Option<usize>, outer: bool) -> Self {
        Self {
            encoder,
            expected,
            count: 0,
            pending_key: false,
            outer,
        }
    }
    fn value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        self.count = self.count.checked_add(1).ok_or(Error::Limit)?;
        value.serialize(&mut *self.encoder)
    }
    fn end(self) -> Result<(), Error> {
        if self.pending_key || self.expected.is_some_and(|n| n != self.count) {
            return Err(Error::Limit);
        }
        self.encoder.end(self.expected.is_none())?;
        if self.outer {
            self.encoder.end(false)?;
        }
        Ok(())
    }
}
macro_rules! unsigned {
    ($method:ident,$ty:ty,$tag:expr) => {
        fn $method(self, v: $ty) -> Result<(), Error> {
            self.integer($tag, v as u128)
        }
    };
}
macro_rules! signed {
    ($method:ident,$ty:ty,$tag:expr) => {
        fn $method(self, v: $ty) -> Result<(), Error> {
            let v = v as i128;
            self.integer($tag, ((v as u128) << 1) ^ ((v >> 127) as u128))
        }
    };
}
impl<'a, 'w, W: io::Write> ser::Serializer for &'a mut Encoder<'w, W> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Compound<'a, 'w, W>;
    type SerializeTuple = Compound<'a, 'w, W>;
    type SerializeTupleStruct = Compound<'a, 'w, W>;
    type SerializeTupleVariant = Compound<'a, 'w, W>;
    type SerializeMap = Compound<'a, 'w, W>;
    type SerializeStruct = Compound<'a, 'w, W>;
    type SerializeStructVariant = Compound<'a, 'w, W>;
    fn is_human_readable(&self) -> bool {
        true
    }
    fn serialize_bool(self, v: bool) -> Result<(), Error> {
        self.tag(if v { 2 } else { 1 })
    }
    signed!(serialize_i8, i8, 3);
    signed!(serialize_i16, i16, 4);
    signed!(serialize_i32, i32, 5);
    signed!(serialize_i64, i64, 6);
    signed!(serialize_i128, i128, 7);
    unsigned!(serialize_u8, u8, 8);
    unsigned!(serialize_u16, u16, 9);
    unsigned!(serialize_u32, u32, 10);
    unsigned!(serialize_u64, u64, 11);
    unsigned!(serialize_u128, u128, 12);
    fn serialize_f32(self, v: f32) -> Result<(), Error> {
        self.f32_bits(v, 0xa0, 0x80)
    }
    fn serialize_f64(self, v: f64) -> Result<(), Error> {
        self.f64_bits(v)
    }
    fn serialize_char(self, v: char) -> Result<(), Error> {
        self.integer(15, v as u128)
    }
    fn serialize_str(self, v: &str) -> Result<(), Error> {
        self.tag(16)?;
        self.text(v)
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<(), Error> {
        self.tag(17)?;
        self.size(v.len())?;
        self.bytes(v)
    }
    fn serialize_none(self) -> Result<(), Error> {
        self.tag(18)
    }
    fn serialize_some<T: Serialize + ?Sized>(self, v: &T) -> Result<(), Error> {
        self.tag(19)?;
        self.nested(v)
    }
    fn serialize_unit(self) -> Result<(), Error> {
        self.tag(20)
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<(), Error> {
        self.tag(20)
    }
    fn serialize_unit_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
    ) -> Result<(), Error> {
        self.serialize_str(variant)
    }
    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        _: &'static str,
        v: &T,
    ) -> Result<(), Error> {
        self.nested(v)
    }
    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        v: &T,
    ) -> Result<(), Error> {
        self.container(true, Some(1))?;
        variant.serialize(&mut *self)?;
        self.nested(v)?;
        self.end(false)
    }
    fn serialize_seq(self, n: Option<usize>) -> Result<Self::SerializeSeq, Error> {
        self.container(false, n)?;
        Ok(Compound::new(self, n, false))
    }
    fn serialize_tuple(self, n: usize) -> Result<Self::SerializeTuple, Error> {
        self.container(false, Some(n))?;
        Ok(Compound::new(self, Some(n), false))
    }
    fn serialize_tuple_struct(
        self,
        _: &'static str,
        n: usize,
    ) -> Result<Self::SerializeTupleStruct, Error> {
        self.serialize_tuple(n)
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        n: usize,
    ) -> Result<Self::SerializeTupleVariant, Error> {
        self.container(true, Some(1))?;
        variant.serialize(&mut *self)?;
        self.container(false, Some(n))?;
        Ok(Compound::new(self, Some(n), true))
    }
    fn serialize_map(self, n: Option<usize>) -> Result<Self::SerializeMap, Error> {
        self.container(true, n)?;
        Ok(Compound::new(self, n, false))
    }
    fn serialize_struct(self, _: &'static str, n: usize) -> Result<Self::SerializeStruct, Error> {
        self.container(true, Some(n))?;
        Ok(Compound::new(self, Some(n), false))
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        n: usize,
    ) -> Result<Self::SerializeStructVariant, Error> {
        self.container(true, Some(1))?;
        variant.serialize(&mut *self)?;
        self.container(true, Some(n))?;
        Ok(Compound::new(self, Some(n), true))
    }
    fn collect_str<T: fmt::Display + ?Sized>(self, value: &T) -> Result<(), Error> {
        struct Count(usize);
        impl fmt::Write for Count {
            fn write_str(&mut self, s: &str) -> fmt::Result {
                self.0 = self.0.checked_add(s.len()).ok_or(fmt::Error)?;
                Ok(())
            }
        }
        struct Output<'a, W> {
            writer: &'a mut W,
            count: usize,
            error: Option<io::Error>,
        }
        impl<W: io::Write> fmt::Write for Output<'_, W> {
            fn write_str(&mut self, s: &str) -> fmt::Result {
                self.writer.write_all(s.as_bytes()).map_err(|e| {
                    self.error = Some(e);
                    fmt::Error
                })?;
                self.count = self.count.checked_add(s.len()).ok_or(fmt::Error)?;
                Ok(())
            }
        }
        let mut count = Count(0);
        fmt::write(&mut count, format_args!("{value}")).map_err(|_| Error::Limit)?;
        self.tag(16)?;
        self.size(count.0)?;
        let mut output = Output {
            writer: self.writer,
            count: 0,
            error: None,
        };
        fmt::write(&mut output, format_args!("{value}"))
            .map_err(|_| output.error.take().map_or(Error::Limit, Error::Io))?;
        if output.count != count.0 {
            return Err(Error::Limit);
        }
        Ok(())
    }
}
macro_rules! sequence {
    ($trait:ident,$method:ident) => {
        impl<W: io::Write> ser::$trait for Compound<'_, '_, W> {
            type Ok = ();
            type Error = Error;
            fn $method<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), Error> {
                self.value(v)
            }
            fn end(self) -> Result<(), Error> {
                Compound::end(self)
            }
        }
    };
}
sequence!(SerializeSeq, serialize_element);
sequence!(SerializeTuple, serialize_element);
sequence!(SerializeTupleStruct, serialize_field);
sequence!(SerializeTupleVariant, serialize_field);
impl<W: io::Write> ser::SerializeMap for Compound<'_, '_, W> {
    type Ok = ();
    type Error = Error;
    fn serialize_key<T: Serialize + ?Sized>(&mut self, k: &T) -> Result<(), Error> {
        if self.pending_key {
            return Err(Error::Limit);
        }
        k.serialize(&mut *self.encoder)?;
        self.pending_key = true;
        Ok(())
    }
    fn serialize_value<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), Error> {
        if !self.pending_key {
            return Err(Error::Limit);
        }
        self.value(v)?;
        self.pending_key = false;
        Ok(())
    }
    fn end(self) -> Result<(), Error> {
        Compound::end(self)
    }
}
macro_rules! structure {
    ($trait:ident) => {
        impl<W: io::Write> ser::$trait for Compound<'_, '_, W> {
            type Ok = ();
            type Error = Error;
            fn serialize_field<T: Serialize + ?Sized>(
                &mut self,
                key: &'static str,
                v: &T,
            ) -> Result<(), Error> {
                key.serialize(&mut *self.encoder)?;
                self.value(v)
            }
            fn end(self) -> Result<(), Error> {
                Compound::end(self)
            }
        }
    };
}
structure!(SerializeStruct);
structure!(SerializeStructVariant);
#[cfg(test)]
#[path = "source_state_binding_tests.rs"]
mod tests;
