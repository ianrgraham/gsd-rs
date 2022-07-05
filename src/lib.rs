pub mod fl;
pub mod hoomd;

mod tests;

#[repr(u8)]
#[derive(Debug, PartialEq)]
enum GSDType {
    UINT8 = 1,
    UINT16,
    UINT32,
    UINT64,
    INT8,
    INT16,
    INT32,
    INT64,
    FLOAT,
    DOUBLE
}

impl GSDType {
    fn from_type<T>() -> Self {
        let type_name = std::any::type_name::<T>();
        match type_name {
            "u8" => GSDType::UINT8,
            "u16" => GSDType::UINT16,
            "u32" => GSDType::UINT32,
            "u64" => GSDType::UINT64,
            "i8" => GSDType::INT8,
            "i16" => GSDType::INT16,
            "i32" => GSDType::INT32,
            "i64" => GSDType::INT64,
            "f32" => GSDType::FLOAT,
            "f64" => GSDType::DOUBLE,
            _ => panic!("Unsupported type")
        }
    }

    fn check_match<T>(&self) -> Result<(), String> {
        let check_type = Self::from_type::<T>();
        if *self != check_type {
            Err(format!("Type mismatch: {:?} != {:?}", self, check_type))
        } else {
            Ok(())
        }
    }
}

#[repr(u8)]
enum OpenFlag {
    Readwrite = 1,
    Readonly = 2,
    Append = 3
}

// TODO - Use Error variants for better debugging
#[allow(dead_code)]
#[repr(i32)]
enum GSDResult {
    Success = 0,
    IO = -1,
    InvalidArgument = -2,
    NotAFile = -3,
    InvalidFileVersion = -4,
    FileCorrupt = -5,
    MemoryAllocFailed = -6,
    NamelistFull = -7,
    NotWritable = -8,
    NotReadable = -9,
}