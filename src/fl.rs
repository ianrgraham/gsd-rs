use std::{ffi::{c_void, CString}, mem::MaybeUninit};

use gsd_sys as libgsd;
use crate::*;

use ndarray::prelude::*;
use log::debug;

/// Raise the appropriate error type.
/// ## Arguments
/// 
///     * retval: Return value from a gsd C API call
///     * extra: Extra string to pass along with the exception
/// 
fn check_gsd_errors(retval: i32, extra: &str) -> Result<(), String>{
    match retval {
        libgsd::gsd_error_GSD_SUCCESS => Ok(()),
        libgsd::gsd_error_GSD_ERROR_IO => Err(format!("I/O error: {}", 2)),
        libgsd::gsd_error_GSD_ERROR_INVALID_ARGUMENT => Err(format!("Invalid gsd argument: {}", extra)),
        libgsd::gsd_error_GSD_ERROR_NOT_A_GSD_FILE => Err(format!("Not a GSD file: {}", extra)),
        libgsd::gsd_error_GSD_ERROR_INVALID_GSD_FILE_VERSION => Err(format!("Unsupported GSD file version: {}", extra)),
        libgsd::gsd_error_GSD_ERROR_MEMORY_ALLOCATION_FAILED => Err(format!("Memory allocation failed: {}", extra)),
        libgsd::gsd_error_GSD_ERROR_FILE_CORRUPT => Err(format!("Corrupt GSD file: {}", extra)),
        libgsd::gsd_error_GSD_ERROR_NAMELIST_FULL=> Err(format!("GSD namelist is full: {}", extra)),
        libgsd::gsd_error_GSD_ERROR_FILE_MUST_BE_WRITABLE => Err(format!("File must be writable: {}", extra)),
        libgsd::gsd_error_GSD_ERROR_FILE_MUST_BE_READABLE => Err(format!("File must be readable: {}", extra)),
        _ => Err(format!("Unknown error: {}", extra)),
    }
}

#[derive(Default)]
pub struct GSDFile {
    name: String,
    mode: String,
    application: String,
    schema: String,
    schema_version: (u32, u32),
    is_open: bool,
    handle: libgsd::gsd_handle
}

impl GSDFile {

    fn check_open(&self) -> Result<(), String> {
        if !self.is_open {
            return Err("File is not open".to_owned());
        }
        Ok(())
    }

    pub fn nframes(&mut self) -> usize {
        let retval = unsafe { libgsd::gsd_get_nframes(&mut self.handle as *mut libgsd::gsd_handle) };
        retval as usize
    }

    pub fn write_chunk<'a, T, D, const I: usize>(&mut self, name: &str, data: D) -> Result<(), String>
        where D : Into<ArrayView<'a, T, Dim<[usize; I]>>>,
            T : 'a + Clone + num_traits::Num,
            Dim<[usize; I]>: Dimension

    {
        self.check_open()?;

        let data: ArrayView<T, Dim<[usize; I]>> = data.into();
        let dim = data.raw_dim();
        let mut N = 0;
        let mut M = 0;
        println!("raw_dim: {:?}", dim);
        if dim.ndim() > 2 {
            return Err(format!("GSD can only write 1 or 2 dimensional arrays: {}", name));
        }
        else if dim.ndim() == 2 { N = dim[0]; M = dim[1]; }
        else { N = dim[0]; M = 1; }

        let data = data.as_standard_layout();

        let gsd_type = GSDType::from_type::<T>();
        let c_name = std::ffi::CString::new(name).expect("CString::new failed");

        let retval = unsafe { libgsd::gsd_write_chunk(
            &mut self.handle as *mut libgsd::gsd_handle,
            c_name.as_ptr(),
            gsd_type as u32,
            N as u64,
            M as u32,
            0,
            data.as_ptr() as *const c_void
        )};

        check_gsd_errors(retval, &self.name)
    }

    pub fn end_frame(&mut self) -> Result<(), String> {
        
        self.check_open()?;

        debug!("end frame: {}", self.name);
    
        let retval = unsafe { libgsd::gsd_end_frame(
            &mut self.handle as *mut libgsd::gsd_handle
        ) };
        check_gsd_errors(retval, &self.name)
    }

}

impl Drop for GSDFile {
    fn drop(&mut self) {
        if self.is_open {
            debug!("Closing file: {}", self.name);
            let retval = unsafe { libgsd::gsd_close(
                &mut self.handle as *mut libgsd::gsd_handle
            ) };
            check_gsd_errors(retval, &self.name).unwrap();
        }
    }
}

pub fn open(name: String, mode: &str, application: String, schema: String, schema_version: (u32, u32)) -> Result<GSDFile, String> {
    let mut exclusive_create = 0i32;
    let mut overwrite = false;

    let c_flags = match mode {
        "wb" => {
            overwrite = true;
            OpenFlag::Append
        }
        "wb+" => {
            overwrite = true;
            OpenFlag::Readwrite
        }
        "rb" => OpenFlag::Readonly,
        "rb+" => OpenFlag::Readwrite,
        "xb" => {
            overwrite = true;
            exclusive_create = 1;
            OpenFlag::Append
        }
        "xb+" => {
            overwrite = true;
            exclusive_create = 1;
            OpenFlag::Readwrite
        }
        "ab" => OpenFlag::Append,
        _ => {return Err("mode must be 'wb', 'wb+', 'rb', 'rb+', 'xb', 'xb+', or 'ab'".to_owned())}
    };

    let mut handle = MaybeUninit::<libgsd::gsd_handle>::uninit();
    let mut raw_handle = unsafe { *handle.as_ptr() };

    let retval = if overwrite {
        let c_name = std::ffi::CString::new(name.clone()).expect("CString::new failed");
        let c_application = std::ffi::CString::new(application.clone()).expect("CString::new failed");
        let c_schema = std::ffi::CString::new(schema.clone()).expect("CString::new failed");

        let c_schema_version = unsafe { libgsd::gsd_make_version(schema_version.0, schema_version.1) };

        unsafe { 
            libgsd::gsd_create_and_open(
                &mut raw_handle as *mut libgsd::gsd_handle,
                c_name.as_ptr(),
                c_application.as_ptr(),
                c_schema.as_ptr(),
                c_schema_version,
                c_flags as u32,
                exclusive_create
            )
        }
    }
    else {
        let c_name = std::ffi::CString::new(name.clone()).expect("CString::new failed");

        unsafe { libgsd::gsd_open(&mut raw_handle as *mut libgsd::gsd_handle, c_name.as_ptr(), c_flags as u32) }
    };
    
    check_gsd_errors(retval, &name);

    let mode = mode.to_owned();
    Ok( GSDFile { name, mode, application, schema, schema_version, handle: raw_handle, is_open: true} )
}