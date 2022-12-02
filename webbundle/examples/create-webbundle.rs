use anyhow::Result;
use headers::ContentType;
use std::io::BufWriter;
use webbundle::{Bundle, Exchange, Version};

// This creates a webbundle which can be used in
// "Navigate-to-WebBundle" feature, explained at
// https://web.dev/web-bundles/.
//
// 1. Run this example: e.g. cargo run --example create-webbundle
// 2. Enable the runtime flag in Chrome: about://flags/#web-bundles
// 3. Drag and drop the generated bundle file (<crate_root>/examples/create-example.wbn) into Chrome.
// 4. You should see "Hello".
fn main() -> Result<()> {
    let bundle = Bundle::builder()
        .version(Version::VersionB2)
        .primary_url("https://example.com/".parse()?)
        .exchange(Exchange::from((
            "https://example.com/".to_string(),
            "Hello".to_string().into_bytes(),
            ContentType::html(),
        )))
        .build()?;

    let out_path =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/create-webbundle.wbn");
    bundle.write_to(BufWriter::new(std::fs::File::create(out_path)?))?;
    Ok(())
}
