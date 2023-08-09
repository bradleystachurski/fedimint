fn main() {
    pub const CODE_VERSION: &str = env!("FEDIMINT_BUILD_CODE_VERSION");
    println!("Hello, world!");
    println!("CODE_VERSION: {}", CODE_VERSION);
}
