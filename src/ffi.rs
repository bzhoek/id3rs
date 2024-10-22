use crate::ID3rs;
use std::ffi::{c_char, CStr};

#[no_mangle]
pub extern "C" fn id3_read(file: *const c_char) -> *mut ID3rs {
  let file = unsafe {
    assert!(!file.is_null());
    CStr::from_ptr(file).to_str().unwrap()
  };
  let id3 = Box::new(ID3rs::read(file).unwrap());
  Box::into_raw(id3)
}

#[no_mangle]
pub extern "C" fn id3_write(ptr: *mut ID3rs, file: *const c_char) {
  assert!(!ptr.is_null());
  let id3rs = unsafe {
    &mut *ptr
  };
  let file = unsafe {
    assert!(!file.is_null());
    CStr::from_ptr(file).to_str().unwrap()
  };
  id3rs.write_to(file).unwrap();
}

#[no_mangle]
pub extern "C" fn id3_set_popularity(ptr: *mut ID3rs, email: *const c_char, rating: u8) {
  let id3rs = unsafe {
    assert!(!ptr.is_null());
    &mut *ptr
  };
  let email = unsafe {
    assert!(!email.is_null());
    CStr::from_ptr(email).to_str().unwrap()
  };
  id3rs.set_popularity(email, rating);
}

#[no_mangle]
pub extern "C" fn id3_free(ptr: *mut ID3rs) {
  if ptr.is_null() {
    return;
  }
  unsafe {
    drop(Box::from_raw(ptr));
  }
}