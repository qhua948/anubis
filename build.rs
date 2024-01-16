use slint_build::CompilerConfiguration;

fn main() {
    let cfg = CompilerConfiguration::new().with_style("material".to_owned());
    slint_build::compile_with_config("ui/home.slint", cfg).unwrap();
}
