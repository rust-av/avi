#![no_main]
#[macro_use] extern crate libfuzzer_sys;
extern crate avi;

use avi::header;

fuzz_target!(|data: &[u8]| {
    let _ = header(data);
});
