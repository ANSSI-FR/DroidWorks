use lazy_static::lazy_static;
use regex::Regex;

pub(crate) fn is_dex(filename: &str) -> bool {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^(dex/)?classes[0-9]*\.dex$")
            .expect("failed to compile dex filename regex");
    }
    RE.is_match(filename)
}

pub(crate) fn is_manifest(filename: &str) -> bool {
    filename == "AndroidManifest.xml" || filename == "manifest/AndroidManifest.xml"
}

pub(crate) fn is_resources(filename: &str) -> bool {
    filename == "resources.arsc" || filename == "res/resources.arsc"
}
