use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::ptr;

use native_windows_gui as nwg;
use winapi::shared::basetsd::UINT_PTR;
use winapi::shared::windef::{HDC, HWND};
use winapi::um::wingdi::{
    CreateFontW, SetBkColor, SetTextColor, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET,
    NULL_BRUSH, OUT_DEFAULT_PRECIS, RGB, VARIABLE_PITCH,
};
use winapi::um::winuser::{
    GetParent, GetSysColor, LoadCursorW, SetCursor, COLOR_BTNFACE, IDC_HAND, WM_CTLCOLORSTATIC,
    WM_SETCURSOR,
};

use crate::{dpi_adjust_size, FONT_FAMILY, FONT_SIZE};

/// Builds an underlined font for the link label.
pub fn build_link_label_font() -> nwg::Font {
    let family_name = OsStr::new(FONT_FAMILY)
        .encode_wide()
        .chain(Some(0u16).into_iter())
        .collect::<Vec<u16>>();
    let family_name_ptr = family_name.as_ptr();

    let size = -dpi_adjust_size(FONT_SIZE as i32);

    let handle = unsafe {
        CreateFontW(
            size,
            0,
            0,
            0,
            0,
            0,
            1, // fdwUnderline
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            VARIABLE_PITCH,
            family_name_ptr,
        )
    };

    nwg::Font { handle }
}

pub fn hook_link_label_style(label: &nwg::Label) {
    hook_link_label_color(label);
    hook_link_label_cursor(label);
}

/// Hooks the `WM_CTLCOLORSTATIC` message to draw the label with blue text.
fn hook_link_label_color(label: &nwg::Label) {
    const HANDLER_ID: UINT_PTR = 0x13337;

    let label_hwnd = label.handle.hwnd().unwrap();
    let parent_handle = nwg::ControlHandle::Hwnd(unsafe { GetParent(label_hwnd) });

    nwg::bind_raw_event_handler(&parent_handle, HANDLER_ID, move |_hwnd, msg, w, l| {
        if msg == WM_CTLCOLORSTATIC {
            let hdc = w as HDC;
            let hwnd = l as HWND;

            if hwnd == label_hwnd {
                unsafe {
                    SetBkColor(hdc, GetSysColor(COLOR_BTNFACE));
                    SetTextColor(hdc, RGB(0, 0, 255));
                    return Some(NULL_BRUSH as isize);
                }
            }
        }

        None
    })
    .unwrap();
}

/// Hooks the `WM_SETCURSOR` message to set the hand cursor when hovering a link label.
///
/// It would be easier to set the `hCursor` parameter in `CreateWindowExW` when the label is created.
/// However, native-windows-gui does not (yet) support setting a custom cursor at creation time.
fn hook_link_label_cursor(label: &nwg::Label) {
    const HANDLER_ID: UINT_PTR = 0x13338;

    nwg::bind_raw_event_handler(&label.handle, HANDLER_ID, move |_hwnd, msg, _w, _l| {
        if msg == WM_SETCURSOR {
            unsafe {
                let hcursor = LoadCursorW(ptr::null_mut(), IDC_HAND);
                SetCursor(hcursor);
            }
            return Some(true as isize);
        }

        None
    })
    .unwrap();
}
