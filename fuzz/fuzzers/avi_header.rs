#![no_main]

use avi::header;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = header(data);
});
