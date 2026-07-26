use no_std_tool::{base, module};

base!();

module! {
    pub mod my_test_module {
        pub fn do_something() {
            let _unused = 5;
        }
    }
}

#[test]
fn test_macro_expansion() {
    my_test_module::do_something();
}
