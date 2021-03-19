/*
 * Copyright (c) 2021 gematik GmbH
 * 
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 * 
 *    http://www.apache.org/licenses/LICENSE-2.0
 * 
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
 */

use std::io::Write;
use std::ptr::null_mut;
use std::slice::from_raw_parts;

use libc::*;

use crate::Error;

pub struct OutputBuffer<'a> {
    handle: *mut ffi::xmlOutputBuffer,
    _writer: Box<WrappedWriter<'a>>,
}

pub struct WrappedWriter<'a>(&'a mut dyn Write);

impl<'a> OutputBuffer<'a> {
    pub fn new<W: Write>(writer: &'a mut W) -> Result<Self, Error> {
        unsafe {
            let writer = Box::new(WrappedWriter(writer));

            let context = &*writer as *const _ as *mut c_void;

            let handle = ffi::xmlOutputBufferCreateIO(io_write, io_close, context, null_mut());
            let handle = Error::check_ptr_mut(handle, "xmlOutputBufferCreateIO")?;

            Ok(Self {
                handle,
                _writer: writer,
            })
        }
    }

    pub fn as_ptr(&self) -> *mut ffi::xmlOutputBuffer {
        self.handle
    }
}

impl Drop for OutputBuffer<'_> {
    fn drop(&mut self) {
        unsafe {
            ffi::xmlOutputBufferClose(self.handle);
        }
    }
}

extern "C" fn io_write(context: *mut c_void, buffer: *const c_char, len: c_int) -> c_int {
    unsafe {
        let data = from_raw_parts(buffer as *const u8, len as usize);

        let context = &mut *(context as *mut WrappedWriter<'static>);
        match context.0.write(data) {
            Ok(written) => written as c_int,
            Err(_) => -1,
        }
    }
}

extern "C" fn io_close(context: *mut c_void) -> c_int {
    unsafe {
        let context = &mut *(context as *mut WrappedWriter<'static>);

        match context.0.flush() {
            Ok(()) => 0,
            Err(_) => -1,
        }
    }
}
