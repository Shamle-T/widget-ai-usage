use std::{
    ffi::c_void,
    ptr,
    sync::{Mutex, OnceLock},
    time::Duration,
};

use tauri::{AppHandle, Manager};
use windows_sys::Win32::{
    Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
    Graphics::Gdi::{
        AddFontMemResourceEx, Arc, BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC,
        CreateFontW, CreatePen, CreateRoundRectRgn, CreateSolidBrush, DeleteDC, DeleteObject,
        DrawTextW, Ellipse, EndPaint, GetStockObject, InvalidateRect, LineTo, MoveToEx, RoundRect,
        ScreenToClient, SelectObject, SetArcDirection, SetBkMode, SetTextColor, SetWindowRgn,
        AD_CLOCKWISE, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, DEFAULT_PITCH, DT_CENTER,
        DT_END_ELLIPSIS, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, FF_DONTCARE, FW_SEMIBOLD,
        NULL_BRUSH, NULL_PEN, OUT_DEFAULT_PRECIS, PAINTSTRUCT, PS_SOLID, SRCCOPY, TRANSPARENT,
    },
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        Input::KeyboardAndMouse::{ReleaseCapture, SetCapture},
        WindowsAndMessaging::{
            AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu,
            FindWindowExW, FindWindowW, GetClientRect, GetCursorPos, GetParent, GetWindowLongPtrW,
            GetWindowRect, IsWindow, RegisterClassExW, SetLayeredWindowAttributes, SetParent,
            SetWindowLongPtrW, SetWindowPos, ShowWindow, TrackPopupMenu, CS_HREDRAW, CS_VREDRAW,
            GWL_STYLE, HWND_TOPMOST, LWA_ALPHA, MF_CHECKED, MF_DISABLED, MF_GRAYED, MF_POPUP,
            MF_SEPARATOR, MF_STRING, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_HIDE,
            SW_SHOWNOACTIVATE, TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_ERASEBKGND,
            WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_PAINT, WM_RBUTTONUP, WNDCLASSEXW,
            WS_CHILD, WS_CLIPCHILDREN, WS_CLIPSIBLINGS, WS_EX_LAYERED, WS_EX_NOACTIVATE,
            WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
        },
    },
};

use crate::{
    app_state::{AppState, AppViewState, ProviderStatus, ProviderView},
    config::Theme,
    providers::UsageWindow,
};

const CLASS_NAME: &str = "LimitLensTaskbarWidget";
const ITEM_WIDTH: i32 = 108;
const PANEL_PADDING: i32 = 4;
const RECOVERY_INTERVAL: Duration = Duration::from_secs(8);
const JETBRAINS_MONO: &[u8] = include_bytes!("../../resources/fonts/JetBrainsMono-Regular.ttf");

static RUNTIME: OnceLock<TaskbarRuntime> = OnceLock::new();

struct TaskbarRuntime {
    app: AppHandle,
    data: Mutex<WidgetData>,
}

#[derive(Clone)]
struct ProviderTile {
    id: String,
    name: String,
    window_label: String,
    used_percent: Option<f64>,
    stale: bool,
}

struct WidgetData {
    hwnd: isize,
    parent: isize,
    x: i32,
    y: i32,
    height: i32,
    container_width: i32,
    container_height: i32,
    width: i32,
    taskbar_offset: Option<i32>,
    theme: Theme,
    tiles: Vec<ProviderTile>,
    view: Option<AppViewState>,
    drag: Option<DragState>,
    dirty: bool,
}

struct DragState {
    start_screen_x: i32,
    origin_x: i32,
    moved: bool,
}

impl Default for WidgetData {
    fn default() -> Self {
        Self {
            hwnd: 0,
            parent: 0,
            x: 0,
            y: 0,
            height: 0,
            container_width: 0,
            container_height: 0,
            width: 0,
            taskbar_offset: None,
            theme: Theme::Dark,
            tiles: Vec::new(),
            view: None,
            drag: None,
            dirty: true,
        }
    }
}

pub fn initialize(app: AppHandle) {
    register_private_font();
    let _ = RUNTIME.set(TaskbarRuntime {
        app: app.clone(),
        data: Mutex::new(WidgetData::default()),
    });
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(RECOVERY_INTERVAL).await;
            let _ = app.run_on_main_thread(ensure_window);
        }
    });
}

fn register_private_font() {
    let font_count = 0_u32;
    unsafe {
        AddFontMemResourceEx(
            JETBRAINS_MONO.as_ptr().cast(),
            JETBRAINS_MONO.len() as u32,
            ptr::null(),
            &font_count,
        );
    }
}

pub fn sync(app: &AppHandle, view: &AppViewState) {
    let Some(runtime) = RUNTIME.get() else {
        return;
    };
    let tiles = view
        .providers
        .iter()
        .filter(|provider| provider.enabled && provider.detected)
        .map(provider_tile)
        .collect::<Vec<_>>();
    if let Ok(mut data) = runtime.data.lock() {
        data.width = tiles.len() as i32 * ITEM_WIDTH + PANEL_PADDING * 2;
        data.taskbar_offset = view.taskbar_offset;
        data.theme = view.theme;
        data.tiles = tiles;
        data.view = Some(view.clone());
        data.dirty = true;
    }
    let _ = app.run_on_main_thread(ensure_window);
}

fn provider_tile(provider: &ProviderView) -> ProviderTile {
    let window = selected_window(provider);
    ProviderTile {
        id: provider.id.clone(),
        name: provider.display_name.to_uppercase(),
        window_label: window
            .map(|window| window.display_name.to_uppercase())
            .unwrap_or_else(|| status_label(provider.status.clone()).to_owned()),
        used_percent: window.and_then(|window| window.used_percent),
        stale: matches!(
            provider.status,
            ProviderStatus::Cached | ProviderStatus::Stale
        ),
    }
}

fn selected_window(provider: &ProviderView) -> Option<&UsageWindow> {
    provider
        .selected_window_id
        .as_ref()
        .and_then(|selected| {
            provider
                .windows
                .iter()
                .find(|window| &window.id == selected)
        })
        .or_else(|| provider.windows.first())
}

fn status_label(status: ProviderStatus) -> &'static str {
    match status {
        ProviderStatus::Loading => "LOADING",
        ProviderStatus::Detected => "DETECTED",
        ProviderStatus::Live => "LIVE",
        ProviderStatus::Cached => "CACHED",
        ProviderStatus::Stale => "STALE",
        ProviderStatus::AuthenticationRequired => "SIGN IN",
        ProviderStatus::NotInstalled => "MISSING",
        ProviderStatus::Unsupported => "UNAVAILABLE",
        ProviderStatus::Error => "ERROR",
    }
}

fn ensure_window() {
    let Some(runtime) = RUNTIME.get() else {
        return;
    };
    let Ok(mut data) = runtime.data.lock() else {
        return;
    };
    if data.tiles.is_empty() {
        if data.hwnd != 0 {
            unsafe { ShowWindow(data.hwnd as HWND, SW_HIDE) };
        }
        return;
    }

    let parent = unsafe { find_taskbar() };
    if parent.is_null() {
        return;
    }
    let mut recreate = data.hwnd == 0 || unsafe { IsWindow(data.hwnd as HWND) } == 0;
    let mut parent_changed = false;
    if !recreate && unsafe { GetParent(data.hwnd as HWND) } != parent {
        unsafe { SetParent(data.hwnd as HWND, parent) };
        data.parent = parent as isize;
        parent_changed = true;
    }
    if recreate {
        data.hwnd = unsafe { create_widget_window(parent) } as isize;
        data.parent = parent as isize;
        recreate = data.hwnd == 0;
        data.container_width = 0;
    }
    if recreate {
        return;
    }
    unsafe {
        let geometry_changed = position_widget(&mut data, parent);
        if data.dirty || parent_changed || geometry_changed {
            InvalidateRect(data.hwnd as HWND, ptr::null(), 0);
            data.dirty = false;
        }
        ShowWindow(data.hwnd as HWND, SW_SHOWNOACTIVATE);
    }
}

unsafe fn find_taskbar() -> HWND {
    let class = wide("Shell_TrayWnd");
    FindWindowW(class.as_ptr(), ptr::null())
}

unsafe fn create_widget_window(parent: HWND) -> HWND {
    let class_name = wide(CLASS_NAME);
    let instance = GetModuleHandleW(ptr::null()) as HINSTANCE;
    let class = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(window_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: instance,
        hIcon: ptr::null_mut(),
        hCursor: ptr::null_mut(),
        hbrBackground: ptr::null_mut(),
        lpszMenuName: ptr::null(),
        lpszClassName: class_name.as_ptr(),
        hIconSm: ptr::null_mut(),
    };
    RegisterClassExW(&class);
    let title = wide("LimitLens");
    let hwnd = CreateWindowExW(
        WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_LAYERED,
        class_name.as_ptr(),
        title.as_ptr(),
        WS_POPUP | WS_CLIPSIBLINGS | WS_CLIPCHILDREN,
        0,
        0,
        1,
        1,
        ptr::null_mut(),
        ptr::null_mut(),
        instance,
        ptr::null(),
    );
    if hwnd.is_null() {
        return hwnd;
    }
    SetLayeredWindowAttributes(hwnd, 0, 255, LWA_ALPHA);
    SetWindowPos(
        hwnd,
        HWND_TOPMOST,
        0,
        0,
        0,
        0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
    );
    let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
    SetWindowLongPtrW(hwnd, GWL_STYLE, ((style & !WS_POPUP) | WS_CHILD) as isize);
    SetParent(hwnd, parent);
    hwnd
}

unsafe fn position_widget(data: &mut WidgetData, parent: HWND) -> bool {
    let mut parent_rect = RECT::default();
    if GetClientRect(parent, &mut parent_rect) == 0 {
        return false;
    }
    let taskbar_width = parent_rect.right - parent_rect.left;
    let taskbar_height = parent_rect.bottom - parent_rect.top;
    let height = (taskbar_height - 4).clamp(32, 46);
    let max_x = (taskbar_width - data.width).max(0);
    let default_x = default_position(parent, data.width, taskbar_width);
    let x = data.taskbar_offset.unwrap_or(default_x).clamp(0, max_x);
    let y = ((taskbar_height - height) / 2).max(0);
    let geometry_changed = data.x != x
        || data.y != y
        || data.height != height
        || data.container_width != taskbar_width
        || data.container_height != taskbar_height;
    data.x = x;
    data.y = y;
    data.height = height;
    data.container_width = taskbar_width;
    data.container_height = taskbar_height;

    // Windows 11 composes its XAML taskbar above a small direct child. A taskbar-sized
    // child participates correctly in composition; its region limits paint and input to
    // the widget pixels so the rest of the taskbar remains untouched and clickable.
    SetWindowPos(
        data.hwnd as HWND,
        ptr::null_mut(),
        0,
        0,
        taskbar_width,
        taskbar_height,
        SWP_NOACTIVATE,
    );
    if geometry_changed {
        update_window_region(data);
    }
    geometry_changed
}

unsafe fn update_window_region(data: &WidgetData) {
    let region = CreateRoundRectRgn(
        data.x,
        data.y,
        data.x + data.width + 1,
        data.y + data.height + 1,
        12,
        12,
    );
    if region.is_null() {
        return;
    }
    // SetWindowRgn owns the region after a successful call.
    if SetWindowRgn(data.hwnd as HWND, region, 1) == 0 {
        DeleteObject(region);
    }
}

unsafe fn default_position(parent: HWND, width: i32, taskbar_width: i32) -> i32 {
    let tray_class = wide("TrayNotifyWnd");
    let tray = FindWindowExW(parent, ptr::null_mut(), tray_class.as_ptr(), ptr::null());
    if !tray.is_null() {
        let mut tray_rect = RECT::default();
        if GetWindowRect(tray, &mut tray_rect) != 0 {
            let mut point = POINT {
                x: tray_rect.left,
                y: tray_rect.top,
            };
            if ScreenToClient(parent, &mut point) != 0 {
                return point.x - width - 8;
            }
        }
    }
    taskbar_width - width - 180
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            paint(hwnd);
            0
        }
        WM_LBUTTONDOWN => {
            begin_drag(hwnd);
            0
        }
        WM_MOUSEMOVE => {
            if wparam & 1 != 0 {
                update_drag(hwnd);
            }
            0
        }
        WM_LBUTTONUP => {
            finish_drag(lparam);
            0
        }
        WM_RBUTTONUP => {
            show_context_menu(hwnd);
            0
        }
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}

unsafe fn begin_drag(hwnd: HWND) {
    let Some(runtime) = RUNTIME.get() else {
        return;
    };
    let mut point = POINT::default();
    GetCursorPos(&mut point);
    if let Ok(mut data) = runtime.data.lock() {
        data.drag = Some(DragState {
            start_screen_x: point.x,
            origin_x: data.x,
            moved: false,
        });
    }
    SetCapture(hwnd);
}

unsafe fn update_drag(hwnd: HWND) {
    let Some(runtime) = RUNTIME.get() else {
        return;
    };
    let mut point = POINT::default();
    GetCursorPos(&mut point);
    let Ok(mut data) = runtime.data.lock() else {
        return;
    };
    let Some(drag) = data.drag.as_ref() else {
        return;
    };
    let delta = point.x - drag.start_screen_x;
    if delta.abs() < 4 && !drag.moved {
        return;
    }
    let origin_x = drag.origin_x;
    if let Some(drag) = data.drag.as_mut() {
        drag.moved = true;
    }
    let parent = GetParent(hwnd);
    let mut parent_rect = RECT::default();
    GetClientRect(parent, &mut parent_rect);
    let next_x = (origin_x + delta).clamp(0, (parent_rect.right - data.width).max(0));
    data.x = next_x;
    data.taskbar_offset = Some(next_x);
    update_window_region(&data);
    InvalidateRect(hwnd, ptr::null(), 0);
}

unsafe fn finish_drag(lparam: LPARAM) {
    ReleaseCapture();
    let Some(runtime) = RUNTIME.get() else {
        return;
    };
    let (moved, x, provider_id) = {
        let Ok(mut data) = runtime.data.lock() else {
            return;
        };
        let moved = data.drag.take().is_some_and(|drag| drag.moved);
        let tile_index =
            ((signed_low_word(lparam) - data.x - PANEL_PADDING) / ITEM_WIDTH).max(0) as usize;
        let provider_id = data.tiles.get(tile_index).map(|tile| tile.id.clone());
        (moved, data.x, provider_id)
    };
    let app = runtime.app.clone();
    let Some(state) = app.try_state::<std::sync::Arc<AppState>>() else {
        return;
    };
    let state = state.inner().clone();
    tauri::async_runtime::spawn(async move {
        if moved {
            state.set_taskbar_offset(x).await;
        } else if let Some(provider_id) = provider_id {
            let _ = state.cycle_provider_window(&app, &provider_id).await;
        }
    });
}

fn signed_low_word(value: LPARAM) -> i32 {
    (value as u32 & 0xffff) as i16 as i32
}

unsafe fn show_context_menu(hwnd: HWND) {
    let Some(runtime) = RUNTIME.get() else {
        return;
    };
    let view = runtime.data.lock().ok().and_then(|data| data.view.clone());
    let Some(view) = view else {
        return;
    };
    let menu = create_context_menu(&view);
    if menu.is_null() {
        return;
    }
    let mut point = POINT::default();
    GetCursorPos(&mut point);
    let command = TrackPopupMenu(
        menu,
        TPM_RIGHTBUTTON | TPM_RETURNCMD | TPM_NONOTIFY,
        point.x,
        point.y,
        0,
        hwnd,
        ptr::null(),
    );
    DestroyMenu(menu);
    if let Some(action) = command_action(command as usize, &view) {
        crate::tray::handle_action(&runtime.app, action);
    }
}

unsafe fn create_context_menu(view: &AppViewState) -> *mut c_void {
    let menu = CreatePopupMenu();
    append_item(
        menu,
        1,
        if view.refreshing {
            "Refreshing..."
        } else {
            "Refresh Now"
        },
        view.refreshing,
    );
    AppendMenuW(menu, MF_SEPARATOR, 0, ptr::null());

    let providers = CreatePopupMenu();
    for (index, provider) in view.providers.iter().enumerate() {
        let flags = MF_STRING | if provider.enabled { MF_CHECKED } else { 0 };
        let label = wide(&format!(
            "{} - {}",
            provider.display_name,
            status_label(provider.status.clone())
        ));
        AppendMenuW(providers, flags, 100 + index, label.as_ptr());
    }
    let providers_label = wide("Providers");
    AppendMenuW(menu, MF_POPUP, providers as usize, providers_label.as_ptr());

    let intervals = CreatePopupMenu();
    for (index, minutes) in [1_u8, 2, 5, 10].iter().enumerate() {
        let flags = MF_STRING
            | if view.refresh_interval_minutes == *minutes {
                MF_CHECKED
            } else {
                0
            };
        let label = wide(&format!(
            "{minutes} minute{}",
            if *minutes == 1 { "" } else { "s" }
        ));
        AppendMenuW(intervals, flags, 200 + index, label.as_ptr());
    }
    let interval_label = wide("Refresh Interval");
    AppendMenuW(menu, MF_POPUP, intervals as usize, interval_label.as_ptr());

    let themes = CreatePopupMenu();
    let dark = wide("Dark");
    let light = wide("Light");
    AppendMenuW(
        themes,
        MF_STRING
            | if view.theme == Theme::Dark {
                MF_CHECKED
            } else {
                0
            },
        300,
        dark.as_ptr(),
    );
    AppendMenuW(
        themes,
        MF_STRING
            | if view.theme == Theme::Light {
                MF_CHECKED
            } else {
                0
            },
        301,
        light.as_ptr(),
    );
    let theme_label = wide("Theme");
    AppendMenuW(menu, MF_POPUP, themes as usize, theme_label.as_ptr());

    let autostart = wide("Start with System");
    AppendMenuW(
        menu,
        MF_STRING
            | if view.start_with_system {
                MF_CHECKED
            } else {
                0
            },
        400,
        autostart.as_ptr(),
    );
    AppendMenuW(menu, MF_SEPARATOR, 0, ptr::null());
    let exit = wide("Exit");
    AppendMenuW(menu, MF_STRING, 999, exit.as_ptr());
    menu
}

unsafe fn append_item(menu: *mut c_void, id: usize, label: &str, disabled: bool) {
    let label = wide(label);
    let flags = MF_STRING | if disabled { MF_DISABLED | MF_GRAYED } else { 0 };
    AppendMenuW(menu, flags, id, label.as_ptr());
}

fn command_action(command: usize, view: &AppViewState) -> Option<String> {
    match command {
        1 => Some("refresh".to_owned()),
        100..=199 => view
            .providers
            .get(command - 100)
            .map(|provider| format!("toggle-provider:{}", provider.id)),
        200..=203 => [1_u8, 2, 5, 10]
            .get(command - 200)
            .map(|minutes| format!("interval:{minutes}")),
        300 => Some("theme:dark".to_owned()),
        301 => Some("theme:light".to_owned()),
        400 => Some("autostart".to_owned()),
        999 => Some("quit".to_owned()),
        _ => None,
    }
}

unsafe fn paint(hwnd: HWND) {
    let mut paint = PAINTSTRUCT::default();
    let target = BeginPaint(hwnd, &mut paint);
    let mut rect = RECT::default();
    GetClientRect(hwnd, &mut rect);
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    if width <= 0 || height <= 0 {
        EndPaint(hwnd, &paint);
        return;
    }
    let buffer = CreateCompatibleDC(target);
    let bitmap = CreateCompatibleBitmap(target, width, height);
    let old_bitmap = SelectObject(buffer, bitmap);

    if let Some(runtime) = RUNTIME.get() {
        if let Ok(data) = runtime.data.lock() {
            draw_widget(buffer, width, height, &data);
        }
    }

    BitBlt(target, 0, 0, width, height, buffer, 0, 0, SRCCOPY);
    SelectObject(buffer, old_bitmap);
    DeleteObject(bitmap);
    DeleteDC(buffer);
    EndPaint(hwnd, &paint);
}

unsafe fn draw_widget(hdc: *mut c_void, _width: i32, _height: i32, data: &WidgetData) {
    let palette = Palette::for_theme(data.theme);
    let background = CreateSolidBrush(palette.background);
    let old_brush = SelectObject(hdc, background);
    let old_pen = SelectObject(hdc, GetStockObject(NULL_PEN));
    RoundRect(
        hdc,
        data.x,
        data.y,
        data.x + data.width,
        data.y + data.height,
        12,
        12,
    );
    SelectObject(hdc, old_pen);
    SelectObject(hdc, old_brush);
    DeleteObject(background);

    for (index, tile) in data.tiles.iter().enumerate() {
        let left = data.x + PANEL_PADDING + index as i32 * ITEM_WIDTH;
        if index > 0 {
            let separator = CreatePen(PS_SOLID, 1, palette.border);
            let previous = SelectObject(hdc, separator);
            MoveToEx(hdc, left, data.y + 8, ptr::null_mut());
            LineTo(hdc, left, data.y + data.height - 8);
            SelectObject(hdc, previous);
            DeleteObject(separator);
        }
        draw_tile(hdc, left, data.y, data.height, tile, palette);
    }
}

unsafe fn draw_tile(
    hdc: *mut c_void,
    left: i32,
    top: i32,
    height: i32,
    tile: &ProviderTile,
    palette: Palette,
) {
    let diameter = (height - 8).clamp(28, 36);
    let ring_left = left + 6;
    let ring_top = top + (height - diameter) / 2;
    let center_x = ring_left + diameter / 2;
    let center_y = ring_top + diameter / 2;

    draw_ring(
        hdc,
        ring_left,
        ring_top,
        diameter,
        tile.used_percent,
        if tile.stale {
            palette.stale
        } else {
            palette.progress
        },
        palette.track,
    );
    let usage = tile
        .used_percent
        .map(|percent| format!("{}%", percent.round() as u8))
        .unwrap_or_else(|| "-".to_owned());
    draw_text(
        hdc,
        &usage,
        RECT {
            left: center_x - diameter / 2 + 2,
            top: center_y - 8,
            right: center_x + diameter / 2 - 2,
            bottom: center_y + 8,
        },
        11,
        palette.primary,
        true,
        DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
    );

    let text_left = ring_left + diameter + 6;
    draw_text(
        hdc,
        &tile.name,
        RECT {
            left: text_left,
            top: top + 5,
            right: left + ITEM_WIDTH - 5,
            bottom: top + height / 2 + 4,
        },
        11,
        palette.primary,
        true,
        DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX | DT_END_ELLIPSIS,
    );
    draw_text(
        hdc,
        &tile.window_label,
        RECT {
            left: text_left,
            top: top + height / 2 - 1,
            right: left + ITEM_WIDTH - 5,
            bottom: top + height - 4,
        },
        9,
        palette.secondary,
        false,
        DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX | DT_END_ELLIPSIS,
    );
}

unsafe fn draw_ring(
    hdc: *mut c_void,
    left: i32,
    top: i32,
    diameter: i32,
    percent: Option<f64>,
    progress_color: u32,
    track_color: u32,
) {
    let old_brush = SelectObject(hdc, GetStockObject(NULL_BRUSH));
    let track = CreatePen(PS_SOLID, 3, track_color);
    let old_pen = SelectObject(hdc, track);
    Ellipse(hdc, left, top, left + diameter, top + diameter);
    SelectObject(hdc, old_pen);
    DeleteObject(track);

    if let Some(percent) = percent.map(|value| value.clamp(0.0, 100.0)) {
        if percent > 0.0 {
            let progress = CreatePen(PS_SOLID, 4, progress_color);
            let old_pen = SelectObject(hdc, progress);
            if percent >= 99.95 {
                Ellipse(hdc, left, top, left + diameter, top + diameter);
            } else {
                let radius = diameter as f64 / 2.0;
                let center_x = left as f64 + radius;
                let center_y = top as f64 + radius;
                let start_x = center_x.round() as i32;
                let start_y = top;
                let angle = percent / 100.0 * std::f64::consts::TAU - std::f64::consts::FRAC_PI_2;
                let end_x = (center_x + radius * angle.cos()).round() as i32;
                let end_y = (center_y + radius * angle.sin()).round() as i32;
                SetArcDirection(hdc, AD_CLOCKWISE);
                Arc(
                    hdc,
                    left,
                    top,
                    left + diameter,
                    top + diameter,
                    start_x,
                    start_y,
                    end_x,
                    end_y,
                );
            }
            SelectObject(hdc, old_pen);
            DeleteObject(progress);
        }
    }
    SelectObject(hdc, old_brush);
}

#[allow(clippy::too_many_arguments)]
unsafe fn draw_text(
    hdc: *mut c_void,
    text: &str,
    mut rect: RECT,
    size: i32,
    color: u32,
    semibold: bool,
    format: u32,
) {
    let face = wide("JetBrains Mono");
    let font = CreateFontW(
        -size,
        0,
        0,
        0,
        if semibold { FW_SEMIBOLD as i32 } else { 400 },
        0,
        0,
        0,
        DEFAULT_CHARSET.into(),
        OUT_DEFAULT_PRECIS.into(),
        CLIP_DEFAULT_PRECIS.into(),
        5,
        (DEFAULT_PITCH | FF_DONTCARE).into(),
        face.as_ptr(),
    );
    let old_font = SelectObject(hdc, font);
    SetBkMode(hdc, TRANSPARENT as i32);
    SetTextColor(hdc, color);
    let text = wide(text);
    DrawTextW(hdc, text.as_ptr(), -1, &mut rect, format);
    SelectObject(hdc, old_font);
    DeleteObject(font);
}

#[derive(Clone, Copy)]
struct Palette {
    background: u32,
    border: u32,
    track: u32,
    progress: u32,
    stale: u32,
    primary: u32,
    secondary: u32,
}

impl Palette {
    fn for_theme(theme: Theme) -> Self {
        match theme {
            Theme::Dark => Self {
                background: rgb(37, 34, 31),
                border: rgb(71, 65, 58),
                track: rgb(88, 80, 72),
                progress: rgb(225, 157, 82),
                stale: rgb(158, 116, 75),
                primary: rgb(248, 241, 229),
                secondary: rgb(176, 166, 153),
            },
            Theme::Light => Self {
                background: rgb(239, 234, 225),
                border: rgb(199, 188, 173),
                track: rgb(188, 177, 163),
                progress: rgb(166, 93, 40),
                stale: rgb(181, 132, 94),
                primary: rgb(37, 34, 30),
                secondary: rgb(101, 94, 85),
            },
        }
    }
}

const fn rgb(red: u8, green: u8, blue: u8) -> u32 {
    red as u32 | ((green as u32) << 8) | ((blue as u32) << 16)
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::{command_action, signed_low_word, AppViewState};
    use crate::config::Theme;

    #[test]
    fn decodes_signed_mouse_coordinates() {
        assert_eq!(signed_low_word(24), 24);
        assert_eq!(signed_low_word(0xffff), -1);
    }

    #[test]
    fn maps_fixed_menu_actions() {
        let view = AppViewState {
            version: 1,
            theme: Theme::Dark,
            refresh_interval_minutes: 5,
            start_with_system: false,
            refreshing: false,
            taskbar_offset: None,
            providers: Vec::new(),
        };
        assert_eq!(command_action(1, &view).as_deref(), Some("refresh"));
        assert_eq!(command_action(202, &view).as_deref(), Some("interval:5"));
        assert_eq!(command_action(999, &view).as_deref(), Some("quit"));
    }
}
