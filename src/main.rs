#![windows_subsystem = "windows"]
#![feature(vec_into_raw_parts)]
extern crate winapi;
extern crate kernel32;
extern crate user32;

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::iter::once;

use std::mem;
use std::ptr::null_mut;
use std::io::Error;

use widestring::U16CString;

use kernel32::{GetModuleHandleW, GetLastError};

use winapi::*;

use self::winapi::shared::windef::{
    HWND
};

use self::winapi::shared::minwindef::{
    HINSTANCE__,
    UINT,
    PUINT,
    LRESULT,
    WPARAM,
    LPARAM
};

use self::winapi::um::winuser::{
    RegisterClassW,
    WNDCLASSW,
    DefWindowProcW,
    CreateWindowExW,
    ShowWindow,
    GetMessageW,
    TranslateMessage,
    DispatchMessageW,
    MSG,
    CS_OWNDC,
    CS_HREDRAW,
    CS_VREDRAW,
    CW_USEDEFAULT,
    SW_SHOW,
    SW_RESTORE,
    WS_OVERLAPPEDWINDOW,
    WS_VISIBLE,
    WS_EX_CLIENTEDGE,
    WM_INPUT,
    GetRawInputDeviceList,
    GetRawInputDeviceInfoA,
    RegisterRawInputDevices,
    GetRawInputData,
    PRAWINPUTDEVICELIST,
    RAWINPUTDEVICELIST,
    RAWINPUTDEVICE,
    RAWINPUTHEADER,
    RAWINPUT,
    RAWMOUSE,
    RAWHID,
    RAWKEYBOARD,
    RID_INPUT,
    RIDEV_NOLEGACY,
    RIDI_PREPARSEDDATA,
    RIDI_DEVICENAME,
    RIDI_DEVICEINFO,
};
use winapi::ctypes::{c_void, wchar_t};
use winapi::um::winuser::{PRAWINPUTHEADER, RIDEV_NOHOTKEYS, RIDEV_INPUTSINK, GetRawInputDeviceInfoW};

fn win32_string(value: &str) -> Vec<u16> {
    OsStr::new(value).encode_wide().chain(once(0)).collect()
}

// Logs timestamp, device handle, and mouse data.
fn win32_mouse_proc(
    ri: &RAWINPUT
) -> LRESULT {
    let rm: &RAWMOUSE = unsafe { ri.data.mouse() };
    println!("mouse: {}",
        ri.header.hDevice as usize
    );
    0
}

fn win32_keyboard_proc(
    ri: &RAWINPUT
) -> LRESULT {
    let rk: &RAWKEYBOARD = unsafe { ri.data.keyboard() };
    println!("keyboard: {} {} {} {}",
        rk.Flags, rk.VKey, rk.ExtraInformation, ri.header.hDevice as usize
    );
    0
}

fn win32_hid_proc(
    ri: &RAWINPUT
) -> LRESULT {
    let rh: &RAWHID = unsafe { ri.data.hid() };
    println!("hid");
    0
}

unsafe fn win32_list_devices() {
    let mut sz: UINT = 0;
    let rc = GetRawInputDeviceList(
        null_mut(),
        &mut sz as *mut _,
        mem::size_of::<RAWINPUTDEVICELIST>() as u32
    );

    let mut rids = vec![RAWINPUTDEVICELIST { hDevice: null_mut(), dwType: u32::MAX }; sz as usize]
        .into_raw_parts();
    let rc = GetRawInputDeviceList(
        rids.0 as *mut _,
        &mut sz as *mut _,
        mem::size_of::<RAWINPUTDEVICELIST>() as u32
    );

    // Find a more elegant way to decompose and recompose the vec.
    let mut rids = Vec::from_raw_parts(rids.0, rids.1, rids.2);
    for i in 0..(sz as usize) {
        println!("hDev: {} type: {}", rids[i].hDevice as usize, rids[i].dwType);

        // TODO: Need a good way to handle wchar_t from API.
        // Documentation implies U16CString is the best way, but checked conversion to utf-8 is
        // desirable.
        let mut ssz: UINT = 0;
        let rc = GetRawInputDeviceInfoW(
            rids[i].hDevice,
            RIDI_DEVICENAME,
            null_mut(),
            &mut ssz as *mut _
        );

        let mut name = vec![0 as wchar_t; ssz as usize].into_raw_parts();
        let rc = GetRawInputDeviceInfoW(
            rids[i].hDevice,
            RIDI_DEVICENAME,
            name.0 as *mut _,
            &mut ssz as *mut _
        );

        let mut name = Vec::from_raw_parts(name.0, name.1, name.2);
        let s: U16CString = U16CString::from_vec_with_nul(name).unwrap();
        println!("{}", s.to_string_lossy());
    }
}

unsafe extern "system" fn win32_wndproc(
    hWnd: HWND,
    message: UINT,
    wParam: WPARAM,
    lParam: LPARAM
) -> LRESULT {
    match message {
        WM_INPUT => {
            // This works: RAWINPUT is the largest size possible.
            let mut dwsize: UINT = mem::size_of::<RAWINPUT>() as u32;
            let mut ri: RAWINPUT = mem::uninitialized();

            let rc = GetRawInputData(
                lParam as *mut _,
                RID_INPUT,
                &mut ri as *mut _ as *mut c_void,
                &mut dwsize as *mut _,
                mem::size_of::<RAWINPUTHEADER>() as u32
            );

            // TODO: Constants are wrong? Find way to check.
            match ri.header.dwType {
                RID_TYPEMOUSE => win32_keyboard_proc(&ri),
                RID_TYPEKEYBOARD => win32_mouse_proc(&ri),
                RID_TYPEHID => win32_hid_proc(&ri)
            }
        },
        _ => DefWindowProcW(hWnd, message, wParam, lParam)
    }
}

fn main() {
    unsafe {
        let name = win32_string("win32_rawinput");
        let title = win32_string("win32_rawinput");

        let hinstance = GetModuleHandleW(null_mut()) as
            *mut winapi::shared::minwindef::HINSTANCE__;

        let wnd_class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW, // CS_OWNDC
            lpfnWndProc: Some(win32_wndproc),
            hInstance: hinstance,
            lpszClassName: name.as_ptr(),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hIcon: null_mut(),
            hCursor: null_mut(),
            hbrBackground: null_mut(),
            lpszMenuName: null_mut(),
        };

        RegisterClassW(&wnd_class);
        let handle = CreateWindowExW(
            WS_EX_CLIENTEDGE,
            name.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE, // Remove WS_VISIBLE?
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            null_mut(),
            null_mut(),
            hinstance,
            null_mut()
        );

        let rc = ShowWindow(
            handle, SW_SHOW | SW_RESTORE
        );

        win32_list_devices();

        let rid = RAWINPUTDEVICE {
            usUsagePage: 0x01,
            usUsage: 0x06,
            // No legacy prevents normal WM events. No hotkeys prevents strange behavior for
            // certain system combinations (e.g. alt+tab would have no tab up). Input sink
            // allows the process to get all events.
            dwFlags: RIDEV_NOLEGACY | RIDEV_NOHOTKEYS | RIDEV_INPUTSINK,
            hwndTarget: handle
        };
        let rc = RegisterRawInputDevices(
            &rid,
            1,
            mem::size_of::<RAWINPUTDEVICE>() as u32
        );

        loop {
            let mut message: MSG = mem::uninitialized();
            if GetMessageW(&mut message as *mut MSG,
                           handle, 0, 0) > 0 {
                TranslateMessage(&message as *const MSG);
                DispatchMessageW(&message as *const MSG);
            } else {
                break;
            }
        }
    }
}
