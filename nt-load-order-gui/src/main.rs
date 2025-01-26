#![windows_subsystem = "windows"]

mod app;
mod linklabel;

use muldiv::MulDiv;
use native_windows_gui as nwg;
use nwg::NativeUi;
use winapi::um::winuser::USER_DEFAULT_SCREEN_DPI;

use crate::app::App;

pub const FONT_FAMILY: &str = "Segoe UI";
pub const FONT_SIZE: u32 = 12;

fn main() {
    nwg::init().expect("Failed to init Native Windows GUI");

    let mut font = nwg::Font::default();
    nwg::Font::builder()
        .family(FONT_FAMILY)
        .size_absolute(FONT_SIZE)
        .build(&mut font)
        .expect("Failed to build global default font");
    nwg::Font::set_global_default(Some(font));

    let app = App::build_ui(Default::default()).expect("Failed to build UI");
    app.init();

    nwg::dispatch_thread_events();
}

/// Calculate the absolute size in pixels for the given size in pixels
/// that was designed for Windows' default 96 dpi.
pub fn dpi_adjust_size(size: i32) -> i32 {
    let dpi = unsafe { nwg::dpi() };
    size.mul_div_round(dpi, USER_DEFAULT_SCREEN_DPI)
        .unwrap_or(size)
}
