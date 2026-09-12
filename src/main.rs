//
// [T]HE [G]AME [O]F [L]IFE
//

#![forbid(unsafe_code)]

mod app;
mod life;

fn main() -> Result<(), pixels::Error> {
    env_logger::init();
    app::run()
}
