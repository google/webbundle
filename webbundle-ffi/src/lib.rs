use libc::size_t;
// use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::ptr;
use std::slice;
use webbundle::Bundle;

pub struct WebBundle(Bundle);

/// Construct a new `WebBundle` from the provided `bytes`.
///
/// If the bytes passed in isn't a valid WebBundle representation,
/// this will return a null pointer.
///
/// # Safety
///
/// Make sure you destroy the WebBundle with [`webbundle_destroy()`] once you are
/// done with it.
///
/// [`webbundle_destroy()`]: fn.webbundle_destroy.html
#[no_mangle]
pub unsafe extern "C" fn webbundle_parse(bytes: *const c_char, length: size_t) -> *const WebBundle {
    let slice = slice::from_raw_parts(bytes as *mut u8, length as usize);
    match Bundle::from_bytes(slice) {
        Ok(bundle) => Box::into_raw(Box::new(WebBundle(bundle))),
        Err(_) => ptr::null(),
    }
}

/// Destroy a `WebBundle` once you are done with it.
///
/// # Safety
///
/// The passed `bundle` must be a valid WebBundle created by [`webbundle_parse()`] function.
///
/// [`webbundle_parse()`]: fn.webbundle_parse.html
#[no_mangle]
pub unsafe extern "C" fn webbundle_destroy(bundle: *mut WebBundle) {
    if !bundle.is_null() {
        drop(Box::from_raw(bundle));
    }
}

/// Copy the `bundle`'s primary_url into a user-provided `buffer`,
/// returning the number of bytes copied.
///
/// If there is no primary-url in the bundle, this returns `-1`.
/// If user-provided buffer's length is not enough, this returns `-2`.
///
/// # Safety
///
/// - The passed `bundle` must be a valid WebBundle created by [`webbundle_parse()`] function.
/// - The user-provided `buffer` should have `length` length.
#[no_mangle]
pub unsafe extern "C" fn webbundle_primary_url(
    bundle: *const WebBundle,
    buffer: *mut c_char,
    length: size_t,
) -> c_int {
    if bundle.is_null() {
        return -1;
    }
    let bundle: &Bundle = &((*bundle).0);
    if let Some(uri) = bundle.primary_url() {
        let uri = uri.to_string();

        let buffer: &mut [u8] = slice::from_raw_parts_mut(buffer as *mut u8, length as usize);

        if buffer.len() < uri.len() {
            return -1;
        }

        ptr::copy_nonoverlapping(uri.as_ptr(), buffer.as_mut_ptr(), uri.len());
        uri.len() as c_int
    } else {
        -1
    }
}
