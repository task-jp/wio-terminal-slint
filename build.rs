fn main() {
    #[cfg(feature = "gui")]
    slint_build::compile_with_config(
        "ui/wio_splash.slint",
        slint_build::CompilerConfiguration::new()
            .embed_resources(slint_build::EmbedResourcesKind::EmbedForSoftwareRenderer),
    )
    .unwrap();
}
