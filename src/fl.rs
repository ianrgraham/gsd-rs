use std::{
    ffi::{c_void, CStr, CString},
    mem::MaybeUninit,
    ptr,
};

use crate::*;
use gsd_sys as libgsd;

use log::debug;
use ndarray::prelude::*;

/// Raise the appropriate error type.
/// ## Arguments
///
///     * retval: Return value from a gsd C API call
///     * extra: Extra string to pass along with the exception
///
fn check_gsd_errors(retval: i32, extra: &str) -> Result<(), String> {
    match retval {
        libgsd::gsd_error_GSD_SUCCESS => Ok(()),
        libgsd::gsd_error_GSD_ERROR_IO => Err(format!("I/O error: {}", 2)),
        libgsd::gsd_error_GSD_ERROR_INVALID_ARGUMENT => {
            Err(format!("Invalid gsd argument: {}", extra))
        }
        libgsd::gsd_error_GSD_ERROR_NOT_A_GSD_FILE => Err(format!("Not a GSD file: {}", extra)),
        libgsd::gsd_error_GSD_ERROR_INVALID_GSD_FILE_VERSION => {
            Err(format!("Unsupported GSD file version: {}", extra))
        }
        libgsd::gsd_error_GSD_ERROR_MEMORY_ALLOCATION_FAILED => {
            Err(format!("Memory allocation failed: {}", extra))
        }
        libgsd::gsd_error_GSD_ERROR_FILE_CORRUPT => Err(format!("Corrupt GSD file: {}", extra)),
        libgsd::gsd_error_GSD_ERROR_NAMELIST_FULL => {
            Err(format!("GSD namelist is full: {}", extra))
        }
        libgsd::gsd_error_GSD_ERROR_FILE_MUST_BE_WRITABLE => {
            Err(format!("File must be writable: {}", extra))
        }
        libgsd::gsd_error_GSD_ERROR_FILE_MUST_BE_READABLE => {
            Err(format!("File must be readable: {}", extra))
        }
        _ => Err(format!("Unknown error: {}", extra)),
    }
}

#[derive(Default)]
pub struct GSDFile {
    name: String,
    mode: String,
    handle: libgsd::gsd_handle,
}

impl GSDFile {
    pub fn try_new(
        name: String,
        mode: String,
        application: Option<String>,
        schema: Option<String>,
        schema_version: Option<(u32, u32)>,
    ) -> Result<Self, String> {
        let mut exclusive_create = 0i32;
        let mut overwrite = false;

        let c_flags = match mode.as_str() {
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
            _ => {
                return Err(
                    "mode must be 'wb', 'wb+', 'rb', 'rb+', 'xb', 'xb+', or 'ab'".to_owned(),
                )
            }
        };

        let uninit_handle = MaybeUninit::<libgsd::gsd_handle>::uninit();
        let mut raw_handle = unsafe { *uninit_handle.as_ptr() };

        let retval = if overwrite {
            if application.is_none() || schema.is_none() || schema_version.is_none() {
                return Err(
                    "If overwriting, must specify application, schema, and schema_version"
                        .to_owned(),
                );
            }

            let c_name = CString::new(name.to_owned()).expect("CString::new failed");
            let c_application = CString::new(application.unwrap()).expect("CString::new failed");
            let c_schema = CString::new(schema.unwrap()).expect("CString::new failed");

            let schema_version = schema_version.unwrap();

            let c_schema_version =
                unsafe { libgsd::gsd_make_version(schema_version.0, schema_version.1) };

            unsafe {
                libgsd::gsd_create_and_open(
                    &mut raw_handle as *mut libgsd::gsd_handle,
                    c_name.as_ptr(),
                    c_application.as_ptr(),
                    c_schema.as_ptr(),
                    c_schema_version,
                    c_flags as u32,
                    exclusive_create,
                )
            }
        } else {
            let c_name = CString::new(name.clone()).expect("CString::new failed");

            unsafe {
                libgsd::gsd_open(
                    &mut raw_handle as *mut libgsd::gsd_handle,
                    c_name.as_ptr(),
                    c_flags as u32,
                )
            }
        };

        check_gsd_errors(retval, &name).unwrap();

        let name = name.to_owned();
        let mode = mode.to_owned();
        Ok(GSDFile {
            name,
            mode,
            handle: raw_handle,
        })
    }

    pub fn nframes(&mut self) -> usize {
        let retval =
            unsafe { libgsd::gsd_get_nframes(&mut self.handle as *mut libgsd::gsd_handle) };
        retval as usize
    }

    pub fn truncate(&mut self) {
        let retval = unsafe { libgsd::gsd_truncate(&mut self.handle as *mut libgsd::gsd_handle) };

        check_gsd_errors(retval, &self.name).unwrap();
    }

    pub fn write_chunk<'a, T, D, const I: usize>(
        &mut self,
        name: &str,
        data: D,
    ) -> Result<(), String>
    where
        D: Into<ArrayView<'a, T, Dim<[usize; I]>>>,
        T: 'a + Clone + num_traits::Num,
        Dim<[usize; I]>: Dimension,
    {
        let data: ArrayView<T, Dim<[usize; I]>> = data.into();
        let dim = data.raw_dim();
        let n;
        let m;
        if dim.ndim() > 2 {
            return Err(format!(
                "GSD can only write 1 or 2 dimensional arrays: {}",
                name
            ));
        } else if dim.ndim() == 2 {
            n = dim[0];
            m = dim[1];
        } else {
            n = dim[0];
            m = 1;
        }

        let data = data.as_standard_layout();

        let gsd_type = GSDType::from_type::<T>();
        let c_name = CString::new(name).expect("CString::new failed");

        let retval = unsafe {
            libgsd::gsd_write_chunk(
                &mut self.handle as *mut libgsd::gsd_handle,
                c_name.as_ptr(),
                gsd_type as u32,
                n as u64,
                m as u32,
                0,
                data.as_ptr() as *const c_void,
            )
        };

        check_gsd_errors(retval, &self.name)
    }

    pub fn end_frame(&mut self) -> Result<(), String> {
        debug!("end frame: {}", self.name);

        let retval = unsafe { libgsd::gsd_end_frame(&mut self.handle as *mut libgsd::gsd_handle) };
        check_gsd_errors(retval, &self.name)
    }

    pub fn chunk_exists(&mut self, frame: usize, name: &str) -> bool {
        let c_name = CString::new(name).expect("CString::new failed");
        let index_entry = unsafe {
            libgsd::gsd_find_chunk(
                &mut self.handle as *mut libgsd::gsd_handle,
                frame as u64,
                c_name.as_ptr(),
            )
        };
        !index_entry.is_null()
    }

    pub fn read_chunk<T: Clone + num_traits::Num>(
        &mut self,
        frame: usize,
        name: &str,
    ) -> Result<Array2<T>, String> {
        let c_name = CString::new(name).expect("CString::new failed");
        if let Some(index_entry) = unsafe {
            libgsd::gsd_find_chunk(
                &mut self.handle as *mut libgsd::gsd_handle,
                frame as u64,
                c_name.as_ptr(),
            )
            .as_ref()
        } {
            let gsd_type: GSDType = unsafe { ::std::mem::transmute(index_entry.type_) };
            gsd_type.check_match::<T>()?;
            let data = Array2::<T>::zeros((index_entry.N as usize, index_entry.M as usize));

            let retval = unsafe {
                libgsd::gsd_read_chunk(
                    &mut self.handle as *mut libgsd::gsd_handle,
                    data.as_ptr() as *mut c_void,
                    index_entry as *const libgsd::gsd_index_entry,
                )
            };

            check_gsd_errors(retval, &self.name)?;

            Ok(data)
        } else {
            Err(format!(
                "frame {} / chunk {} not found in: {}",
                frame, name, self.name
            ))
        }
    }

    pub fn read_chunk_dyn<T: Clone + num_traits::Num>(
        &mut self,
        frame: usize,
        name: &str,
    ) -> Result<ArrayD<T>, String> {
        let data = self.read_chunk(frame, name)?;
        Ok(data.into_dyn())
    }

    pub fn read_chunk_with_dim<T: Clone + num_traits::Num, const I: usize>(
        &mut self,
        frame: usize,
        name: &str,
    ) -> Result<Array<T, Dim<[usize; I]>>, String>
    where
        Dim<[usize; I]>: Dimension,
    {
        let data = self.read_chunk::<T>(frame, name)?;
        let cols = data.ncols();
        assert!(I == cols);
        match data.into_dimensionality::<Dim<[usize; I]>>() {
            Ok(data) => Ok(data),
            Err(e) => Err(format!("{}", e)),
        }
    }

    pub fn find_matching_chunk_names(&mut self, pattern: &str) -> Vec<&str> {
        let mut result = Vec::<&str>::new();
        let c_pattern = CString::new(pattern).expect("CString::new failed");
        let null_ptr: *const i8 = ptr::null();

        let mut c_found = unsafe {
            CStr::from_ptr(libgsd::gsd_find_matching_chunk_name(
                &mut self.handle as *mut libgsd::gsd_handle,
                c_pattern.as_ptr(),
                null_ptr,
            ))
        };

        while !c_found.as_ptr().is_null() {
            result.push(c_found.to_str().unwrap());
            c_found = unsafe {
                CStr::from_ptr(libgsd::gsd_find_matching_chunk_name(
                    &mut self.handle as *mut libgsd::gsd_handle,
                    c_pattern.as_ptr(),
                    c_found.as_ptr(),
                ))
            };
        }

        return result;
    }

    pub fn upgrade(&mut self) -> Result<(), String> {
        let retval = unsafe { libgsd::gsd_upgrade(&mut self.handle as *mut libgsd::gsd_handle) };

        check_gsd_errors(retval, &self.name)?;

        Ok(())
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn mode(&self) -> &str {
        &self.mode
    }

    pub fn gsd_version(&self) -> (u32, u32) {
        let v = self.handle.header.gsd_version;
        return (v >> 16, v & 0xffff);
    }

    pub fn schema_version(&self) -> (u32, u32) {
        let v = self.handle.header.schema_version;
        return (v >> 16, v & 0xffff);
    }

    pub fn schema(&self) -> &str {
        let schema = self.handle.header.schema;
        return unsafe { CStr::from_ptr(schema.as_ptr()).to_str().unwrap() };
    }

    pub fn application(&self) -> &str {
        let application = self.handle.header.application;
        return unsafe { CStr::from_ptr(application.as_ptr()).to_str().unwrap() };
    }
}

impl Drop for GSDFile {
    fn drop(&mut self) {
        debug!("Closing file: {}", self.name);
        let retval = unsafe { libgsd::gsd_close(&mut self.handle as *mut libgsd::gsd_handle) };
        check_gsd_errors(retval, &self.name).unwrap();
    }
}

#[macro_export]
macro_rules! open {
    ($name:expr, $mode:expr) => {
        $crate::fl::GSDFile::try_new($name.to_owned(), $mode.to_owned(), None, None, None)
    };
    ($name:expr, $mode:expr, $app:expr, $schema:expr, $schema_ver:expr) => {
        $crate::fl::GSDFile::try_new(
            $name.to_owned(),
            $mode.to_owned(),
            Some($app.to_owned()),
            Some($schema.to_owned()),
            Some($schema_ver),
        )
    };
}

pub use open;
