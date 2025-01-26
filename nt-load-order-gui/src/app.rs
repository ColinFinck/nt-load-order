use std::{mem, ptr};

use lazy_static::lazy_static;
use native_windows_derive as nwd;
use native_windows_gui as nwg;

use nt_load_order::NtLoadOrder;
use nwd::{NwgPartial, NwgUi};
use nwg::stretch::geometry::{Rect, Size};
use nwg::stretch::style::Dimension;
use nwg::{CheckBox, CheckBoxState, EmbedResource, Icon, RadioButtonState};
use raw_cpuid::CpuId;
use winapi::shared::basetsd::UINT_PTR;
use winapi::shared::minwindef::LOWORD;
use winapi::shared::windef::{POINT, RECT};
use winapi::um::commctrl::LVSCW_AUTOSIZE;
use winapi::um::winuser::{
    GetParent, GetWindowRect, ScreenToClient, SetWindowPos, SWP_NOZORDER, WM_SIZE,
};

use crate::linklabel::{build_link_label_font, hook_link_label_style};
use crate::{dpi_adjust_size, FONT_SIZE};

const APP_TITLE: &str = "nt-load-order-gui";
const WINDOW_TITLE: &str = concat!(
    "nt-load-order gui ",
    env!("CARGO_PKG_VERSION"),
    " - by Colin Finck"
);

const PT_0: Dimension = Dimension::Points(0.0);
const PT_10: Dimension = Dimension::Points(10.0);

const MARGIN_0: Rect<Dimension> = Rect {
    start: PT_0,
    end: PT_0,
    top: PT_0,
    bottom: PT_0,
};

const MARGIN_10: Rect<Dimension> = Rect {
    start: PT_10,
    end: PT_10,
    top: PT_0,
    bottom: PT_10,
};

lazy_static! {
    static ref CPU_VENDOR: Option<String> = CpuId::new()
        .get_vendor_info()
        .map(|vendor_info| vendor_info.as_str().to_owned());
}

#[derive(Default, NwgUi)]
pub struct App {
    #[nwg_control(title: WINDOW_TITLE)]
    #[nwg_events(OnMinMaxInfo: [App::on_min_max_info(SELF, EVT_DATA)], OnWindowClose: [App::on_close])]
    window: nwg::Window,

    #[nwg_layout(parent: window, flex_direction: nwg::stretch::style::FlexDirection::Column)]
    layout: nwg::FlexboxLayout,

    #[nwg_control(flags: "VISIBLE")]
    #[nwg_layout_item(layout: layout, margin: MARGIN_0,
        size: Size { width: Dimension::Auto, height: Dimension::Points(180.0) }
    )]
    frame: nwg::Frame,

    #[nwg_partial(parent: frame)]
    #[nwg_events(
        (source_ui.local_system_root_option, OnButtonClick): [App::on_local_system_root_option_click],
        (source_ui.custom_system_root_option, OnButtonClick): [App::on_custom_system_root_option_click],
        (source_ui.custom_system_root_path, OnMousePress): [App::on_custom_system_root_path_press(SELF, EVT)],
        (steps_ui.sort_by_tag_and_group, OnButtonClick): [App::update_load_order],
        (steps_ui.sort_by_hardcoded_groups, OnButtonClick): [App::update_load_order],
        (steps_ui.sort_by_hardcoded_service_lists, OnButtonClick): [App::update_load_order],
        (steps_ui.add_kernel_binaries, OnButtonClick): [App::update_load_order],
        (steps_ui.add_imports, OnButtonClick): [App::update_load_order],
    )]
    frames: FramesPartial,

    #[nwg_control(list_style: nwg::ListViewStyle::Detailed, ex_flags: nwg::ListViewExFlags::FULL_ROW_SELECT)]
    #[nwg_layout_item(layout: layout, margin: MARGIN_10, flex_grow: 1.0)]
    list: nwg::ListView,

    #[nwg_resource(title: "Select Custom System Root", action: nwg::FileDialogAction::OpenDirectory)]
    select_custom_system_root_dialog: nwg::FileDialog,
}

#[derive(Default, NwgPartial)]
pub struct FramesPartial {
    #[nwg_layout]
    layout: nwg::GridLayout,

    #[nwg_control(flags: "BORDER|VISIBLE")]
    #[nwg_layout_item(layout: layout, row: 0, col: 0)]
    left_frame: nwg::Frame,

    #[nwg_control(flags: "BORDER|VISIBLE")]
    #[nwg_layout_item(layout: layout, row: 0, col: 1)]
    right_frame: nwg::Frame,

    #[nwg_partial(parent: left_frame)]
    source_ui: SourceFramePartial,

    #[nwg_partial(parent: right_frame)]
    steps_ui: StepsFramePartial,
}

#[derive(Default, NwgPartial)]
pub struct SourceFramePartial {
    // Add an extra row for the `custom_system_root_path` label, even though we later position it ourselves.
    #[nwg_layout(max_row: Some(3))]
    grid: nwg::GridLayout,

    #[nwg_control(text: "Use the Local System Root", check_state: nwg::RadioButtonState::Checked)]
    #[nwg_layout_item(layout: grid, row: 0, col: 0)]
    local_system_root_option: nwg::RadioButton,

    #[nwg_control(text: "Use a Custom System Root")]
    #[nwg_layout_item(layout: grid, row: 1, col: 0)]
    custom_system_root_option: nwg::RadioButton,

    #[nwg_control(flags: "NONE")]
    custom_system_root_path: nwg::Label,
}

#[derive(Default, NwgPartial)]
pub struct StepsFramePartial {
    #[nwg_layout]
    grid: nwg::GridLayout,

    #[nwg_control(text: "Sort by Tag and Group", check_state: nwg::CheckBoxState::Checked)]
    #[nwg_layout_item(layout: grid, row: 0, col: 0)]
    sort_by_tag_and_group: nwg::CheckBox,

    #[nwg_control(text: "Sort by hardcoded Groups", check_state: nwg::CheckBoxState::Checked)]
    #[nwg_layout_item(layout: grid, row: 1, col: 0)]
    sort_by_hardcoded_groups: nwg::CheckBox,

    #[nwg_control(text: "Sort by hardcoded Service Lists", check_state: nwg::CheckBoxState::Checked)]
    #[nwg_layout_item(layout: grid, row: 2, col: 0)]
    sort_by_hardcoded_service_lists: nwg::CheckBox,

    #[nwg_control(text: "Add Kernel binaries", check_state: nwg::CheckBoxState::Checked)]
    #[nwg_layout_item(layout: grid, row: 3, col: 0)]
    add_kernel_binaries: nwg::CheckBox,

    #[nwg_control(text: "Add Imports", check_state: nwg::CheckBoxState::Checked)]
    #[nwg_layout_item(layout: grid, row: 4, col: 0)]
    add_imports: nwg::CheckBox,
}

impl App {
    pub fn init(&self) {
        // Load the application icon from the resources.
        let embed = EmbedResource::load(None).unwrap();
        let icon = Icon::from_embed(&embed, Some(1), None).unwrap();
        self.window.set_icon(Some(&icon));

        // Turn the `custom_system_root_path` label into a clickable link label.
        let link_label_font = build_link_label_font();
        self.frames
            .source_ui
            .custom_system_root_path
            .set_font(Some(&link_label_font));
        hook_link_label_style(&self.frames.source_ui.custom_system_root_path);
        self.hook_custom_system_root_path_link_label_position();

        self.list.set_redraw(false);

        // Add list columns.
        self.list.set_headers_enabled(true);
        self.list.insert_column("Group");
        self.list.insert_column("Tag");
        self.list.insert_column("Service");
        self.list.insert_column("Image Path");
        self.list.insert_column("Reason");

        // Add initial data to the list.
        self.update_load_order_inner();

        // Auto-size the list columns based on the data.
        for i in 0..5 {
            self.list.set_column_width(i, LVSCW_AUTOSIZE as isize);
        }

        self.list.set_redraw(true);
    }

    /// Hooks the `WM_SIZE` message to position the `custom_system_root_path` link label below
    /// `custom_system_root_option`.
    fn hook_custom_system_root_path_link_label_position(&self) {
        const HANDLER_ID: UINT_PTR = 0x20001;

        let label = &self.frames.source_ui.custom_system_root_path;
        let radio_button = &self.frames.source_ui.custom_system_root_option;

        let label_hwnd = label.handle.hwnd().unwrap();
        let radio_hwnd = radio_button.handle.hwnd().unwrap();
        let parent_handle = nwg::ControlHandle::Hwnd(unsafe { GetParent(label_hwnd) });
        let parent_hwnd = parent_handle.hwnd().unwrap();

        nwg::bind_raw_event_handler(&parent_handle, HANDLER_ID, move |_hwnd, msg, _w, l| {
            if msg == WM_SIZE {
                unsafe {
                    let new_frame_width = LOWORD(l as u32) as i32;

                    let mut radio_rect: RECT = mem::zeroed();
                    GetWindowRect(radio_hwnd, &mut radio_rect);

                    let mut radio_pt = POINT {
                        x: radio_rect.left,
                        y: radio_rect.top,
                    };
                    ScreenToClient(parent_hwnd, &mut radio_pt);

                    // Increase the height a bit for the underline.
                    let label_height = dpi_adjust_size(FONT_SIZE as i32 + 5);

                    SetWindowPos(
                        label_hwnd,
                        ptr::null_mut(),
                        radio_pt.x + dpi_adjust_size(25),
                        radio_pt.y + dpi_adjust_size(45),
                        new_frame_width - dpi_adjust_size(55),
                        label_height,
                        SWP_NOZORDER,
                    );
                }
            }

            None
        })
        .unwrap();
    }

    fn on_close(&self) {
        nwg::stop_thread_dispatch();
    }

    fn on_custom_system_root_option_click(&self) {
        if !self.select_custom_system_root() {
            self.revert_to_local_system_root();
        }

        self.update_load_order();
    }

    fn on_custom_system_root_path_press(&self, evt: nwg::Event) {
        if evt == nwg::Event::OnMousePress(nwg::MousePressEvent::MousePressLeftUp) && self.select_custom_system_root() {
            self.update_load_order();
        }
    }

    fn on_local_system_root_option_click(&self) {
        self.frames
            .source_ui
            .custom_system_root_path
            .set_visible(false);

        self.update_load_order();
    }

    fn on_min_max_info(&self, data: &nwg::EventData) {
        let data = data.on_min_max();
        data.set_min_size(600, 400);
    }

    fn revert_to_local_system_root(&self) {
        self.frames
            .source_ui
            .local_system_root_option
            .set_check_state(nwg::RadioButtonState::Checked);
        self.frames
            .source_ui
            .custom_system_root_option
            .set_check_state(nwg::RadioButtonState::Unchecked);

        let custom_system_root_path = &self.frames.source_ui.custom_system_root_path;
        custom_system_root_path.set_text("");
        custom_system_root_path.set_visible(false);
    }

    fn select_custom_system_root(&self) -> bool {
        if !self
            .select_custom_system_root_dialog
            .run(Some(&self.window))
        {
            return false;
        }

        let Ok(os_string) = self.select_custom_system_root_dialog.get_selected_item() else {
            return false;
        };

        let Ok(custom_system_root) = os_string.into_string() else {
            return false;
        };

        let custom_system_root_path = &self.frames.source_ui.custom_system_root_path;
        custom_system_root_path.set_text(&custom_system_root);
        custom_system_root_path.set_visible(true);

        true
    }

    fn update_load_order(&self) {
        self.list.set_redraw(false);
        self.update_load_order_inner();
        self.list.set_redraw(true);
    }

    fn update_load_order_inner(&self) {
        let system_root = if let RadioButtonState::Checked = self
            .frames
            .source_ui
            .custom_system_root_option
            .check_state()
        {
            Some(self.frames.source_ui.custom_system_root_path.text())
        } else {
            None
        };

        self.list.clear();

        let load_order = NtLoadOrder::new()
            .system_root(system_root)
            .cpu_vendor(CPU_VENDOR.clone())
            .sort_by_tag_and_group(is_checked(&self.frames.steps_ui.sort_by_tag_and_group))
            .sort_by_hardcoded_groups(is_checked(&self.frames.steps_ui.sort_by_hardcoded_groups))
            .sort_by_hardcoded_service_lists(is_checked(
                &self.frames.steps_ui.sort_by_hardcoded_service_lists,
            ))
            .add_kernel_binaries(is_checked(&self.frames.steps_ui.add_kernel_binaries))
            .add_imports(is_checked(&self.frames.steps_ui.add_imports));

        let entries = match load_order.get() {
            Ok(entries) => entries,
            Err(e) => {
                nwg::modal_error_message(&self.window, APP_TITLE, &e.to_string());
                return;
            }
        };

        for entry in entries {
            self.list.insert_items_row(
                None,
                &[
                    format_option(entry.group.map(|group| group.display_name)),
                    format_option(entry.tag),
                    entry.name,
                    entry.image_path,
                    entry.reason,
                ],
            );
        }
    }
}

fn format_option<U>(option: Option<U>) -> String
where
    U: ToString,
{
    match option {
        Some(value) => value.to_string(),
        None => "<none>".to_string(),
    }
}

fn is_checked(checkbox: &CheckBox) -> bool {
    matches!(checkbox.check_state(), CheckBoxState::Checked)
}
