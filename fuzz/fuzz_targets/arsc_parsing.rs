#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _arsc = dw_resources::resources::parse(data);
});
