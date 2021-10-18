use gsd_sys::*;
use std::ffi::CString;
use std::fs::remove_file;  
use std::io::ErrorKind;
use std::env::temp_dir;

fn get_test_file_name() -> String {
    let rusty_fname = format!(
        "{}/test.gsd",
        temp_dir().into_os_string().into_string().unwrap()
    );
    rusty_fname
}

fn safely_remove_file_if_exists(file: &String) {
    match remove_file(file) {
        Ok(()) => (),
        Err(error) => match error.kind() {
            ErrorKind::NotFound => (),
            other_error => {
                panic!("Problem removing the file: {:?}", other_error)
            }
        }
    }
}

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
            schema_version
        );
        assert_eq!(res, gsd_error_GSD_SUCCESS); // checks that file was created without issue
    }

    safely_remove_file_if_exists(&rusty_fname)
}


fn main() {
    println!("Hello, world!");
    create_and_remove_file();
}
