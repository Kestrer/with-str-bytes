//! Safely manipulate the bytes of a UTF-8 string.
//!
//! This library provides helpers to manipulate the bytes of a UTF-8 string without using `unsafe`.
//! It does not rely on the standard library, and can be used in `no_std` environments.
#![no_std]

#[cfg(test)]
extern crate std;

use core::str;

/// Executes a function on the bytes of a string, asserting that it is valid UTF-8.
///
/// # Panics
///
/// This will panic if the function causes the string to become invalid UTF-8. In this case, the
/// bytes up to the point of the first invalid UTF-8 byte will remain the same, and the contents of
/// the rest of the string is unspecified, although it will be valid UTF-8.
///
/// If the callback itself panics, the entire string's contents is unspecified, but it will be
/// valid UTF-8. Even if the byte slice was set to invalid UTF-8, there will not be a double panic.
///
/// # Examples
///
/// Replace all spaces in a string with dashes in-place:
///
/// ```
/// let mut data: Box<str> = Box::from("Lorem ipsum dolor sit amet");
/// with_str_bytes::with_str_bytes(&mut data, |bytes| {
///     for byte in bytes {
///         if *byte == b' ' {
///             *byte = b'-';
///         }
///     }
/// });
/// assert_eq!(&*data, "Lorem-ipsum-dolor-sit-amet");
/// ```
pub fn with_str_bytes<R, F>(s: &mut str, f: F) -> R
where
    F: FnOnce(&mut [u8]) -> R,
{
    struct Guard<'a> {
        bytes: &'a mut [u8],
        panicking: bool,
    }
    impl Drop for Guard<'_> {
        fn drop(&mut self) {
            if self.panicking {
                for byte in &mut *self.bytes {
                    *byte = 0;
                }
            } else if let Err(e) = str::from_utf8(self.bytes) {
                for byte in &mut self.bytes[e.valid_up_to()..] {
                    *byte = 0;
                }
                panic!("`with_bytes` encountered invalid utf-8: {}", e);
            }
        }
    }

    let mut guard = Guard {
        bytes: unsafe { s.as_bytes_mut() },
        panicking: true,
    };
    let ret = f(&mut guard.bytes);
    guard.panicking = false;
    ret
}

#[cfg(test)]
mod tests {
    use std::boxed::Box;
    use std::panic::{self, AssertUnwindSafe};
    use std::string::String;

    use super::with_str_bytes;

    #[test]
    fn empty() {
        let mut data: Box<str> = Box::from("");
        with_str_bytes(&mut data, |bytes| {
            assert_eq!(bytes, &mut []);
        });
        assert_eq!(&*data, "");
    }

    #[test]
    fn valid_utf8() {
        let initial = "--------------------------";
        let replaced = b"Lorem ipsum dolor sit amet";

        let mut data: Box<str> = Box::from(initial);
        with_str_bytes(&mut data, |bytes| {
            bytes.copy_from_slice(replaced);
        });
        assert_eq!(data.as_bytes(), replaced);
    }

    #[test]
    fn invalid_utf8() {
        let mut data: Box<str> = Box::from("abc");

        let msg = *panic::catch_unwind(AssertUnwindSafe(|| {
            with_str_bytes(&mut data, |bytes| {
                bytes[1] = 0xC0;
            });
        }))
        .unwrap_err()
        .downcast::<String>()
        .unwrap();

        assert_eq!(msg, "`with_bytes` encountered invalid utf-8: invalid utf-8 sequence of 1 bytes from index 1");

        assert_eq!(&*data, "a\0\0");
    }

    #[test]
    fn panics() {
        let mut data: Box<str> = Box::from("abc");

        let msg = *panic::catch_unwind(AssertUnwindSafe(|| {
            with_str_bytes(&mut data, |_| panic!("Oh no"));
        }))
        .unwrap_err()
        .downcast::<&'static str>()
        .unwrap();

        assert_eq!(msg, "Oh no");

        assert_eq!(&*data, "\0\0\0");
    }
}
