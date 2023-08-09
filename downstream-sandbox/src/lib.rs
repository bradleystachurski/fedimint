pub fn print_code_version() {
    pub const CODE_VERSION: &str = env!("FEDIMINT_BUILD_CODE_VERSION");
    println!("CODE_VERSION: {}", CODE_VERSION);
}
