mod gen {
    include!(concat!(env!("OUT_DIR"), "/echo.rs"));
}

pub use gen::echo::*;
