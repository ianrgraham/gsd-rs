#![cfg(test)]

use crate::{fl, hoomd, hoomd_open};
use gsd_sys::*;
use std::env::temp_dir;
use std::ffi::CString;
use std::fs::remove_file;
use std::io::ErrorKind;

fn get_test_file_name() -> String {
    let rusty_fname = format!(
        "{}/test_gsd_fl.gsd",
        temp_dir().into_os_string().into_string().unwrap()
    );
    rusty_fname
}

fn safely_remove_file_if_exists(file: &str) {
    match remove_file(file) {
        Ok(()) => (),
        Err(error) => match error.kind() {
            ErrorKind::NotFound => (),
            other_error => {
                panic!("Problem removing the file: {:?}", other_error)
            }
        },
    }
}

#[test]
fn create_and_remove_file() {
    let rusty_fname = get_test_file_name();

    safely_remove_file_if_exists(&rusty_fname);

    let fname = CString::new(rusty_fname.clone()).expect("CString::new failed");
    let app = CString::new("gsd-sys").expect("CString::new failed");
    let schema = CString::new("test").expect("CString::new failed");
    let schema_version: u32 = 0;

    unsafe {
        let res = gsd_create(
            fname.as_ptr(),
            app.as_ptr(),
            schema.as_ptr(),
            schema_version,
        );
        assert_eq!(res, gsd_error_GSD_SUCCESS); // checks that file was created without issue
    }

    safely_remove_file_if_exists(&rusty_fname)
}

#[test]
fn fl_module_api() {
    let rusty_fname = get_test_file_name();

    let mut gsd_file =
        fl::open!(&rusty_fname, "wb+", "My application", "My Schema", (1, 0)).unwrap();

    let data = vec![1.0f32, 2.0, 3.0, 4.0];
    gsd_file.write_chunk("chunk1", &data).unwrap();
    gsd_file.end_frame().unwrap();

    gsd_file
        .write_chunk("chunk1", &vec![9.0f32, 10.0, 11.0, 12.0])
        .unwrap();
    gsd_file.end_frame().unwrap();

    gsd_file
        .write_chunk("chunk1", &vec![13.0f32, 14.0])
        .unwrap();
    gsd_file.end_frame().unwrap();

    let output = gsd_file.read_chunk::<f32>(2, "chunk1").unwrap();
    assert!(output == ndarray::Array2::from(vec![[13.0f32], [14.0]]));
    assert!(gsd_file.nframes() == 3);

    safely_remove_file_if_exists("file.gsd");
}

#[test]
fn hoomd_module_api() {
    let rusty_fname = get_test_file_name();

    let mut hoomd_file = hoomd_open!(&rusty_fname);
}
