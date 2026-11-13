//! Refresh geometry at invocation, without reading window titles or application content.
use athanor_core::geometry::WorkArea;
type DisplaySnapshot = (Vec<WorkArea>, Option<(i32, i32)>);

pub fn current(fallback: &[WorkArea]) -> DisplaySnapshot {
    #[cfg(target_os = "linux")]
    if let Some(result) = x11(fallback) {
        return result;
    }
    #[cfg(target_os = "windows")]
    if let Some(result) = windows(fallback) {
        return result;
    }
    (fallback.to_vec(), None)
}

#[cfg(target_os = "linux")]
fn x11(fallback: &[WorkArea]) -> Option<DisplaySnapshot> {
    use x11rb::{
        connection::Connection,
        protocol::{
            randr::ConnectionExt as _,
            xproto::{AtomEnum, ConnectionExt as _},
        },
    };
    let (connection, screen) = x11rb::connect(None).ok()?;
    let root = connection.setup().roots[screen].root;
    let atom = |name: &[u8]| {
        connection
            .intern_atom(false, name)
            .ok()?
            .reply()
            .ok()
            .map(|r| r.atom)
    };
    let foreground = (|| {
        let active = connection
            .get_property(
                false,
                root,
                atom(b"_NET_ACTIVE_WINDOW")?,
                AtomEnum::WINDOW,
                0,
                1,
            )
            .ok()?
            .reply()
            .ok()?
            .value32()?
            .next()?;
        let geometry = connection.get_geometry(active).ok()?.reply().ok()?;
        let coords = connection
            .translate_coordinates(active, root, 0, 0)
            .ok()?
            .reply()
            .ok()?;
        Some((
            i32::from(coords.dst_x) + i32::from(geometry.width) / 2,
            i32::from(coords.dst_y) + i32::from(geometry.height) / 2,
        ))
    })();
    let work = (|| {
        let reply = connection
            .get_property(
                false,
                root,
                atom(b"_NET_WORKAREA")?,
                AtomEnum::CARDINAL,
                0,
                4,
            )
            .ok()?
            .reply()
            .ok()?;
        let values: Vec<_> = reply.value32()?.collect();
        (values.len() == 4).then_some(values)
    })();
    let monitors = connection
        .randr_get_monitors(root, true)
        .ok()?
        .reply()
        .ok()?;
    let mut areas = Vec::new();
    for monitor in monitors.monitors {
        let name = connection.get_atom_name(monitor.name).ok()?.reply().ok()?;
        let id = String::from_utf8_lossy(&name.name).into_owned();
        let scale = fallback
            .iter()
            .find(|area| area.id == id)
            .or_else(|| fallback.first())
            .map_or(1.0, |area| area.scale);
        let mut x = i32::from(monitor.x);
        let mut y = i32::from(monitor.y);
        let mut right = x + i32::from(monitor.width);
        let mut bottom = y + i32::from(monitor.height);
        if let Some(work) = &work {
            x = x.max(work[0] as i32);
            y = y.max(work[1] as i32);
            right = right.min((i64::from(work[0] as i32) + i64::from(work[2])) as i32);
            bottom = bottom.min((i64::from(work[1] as i32) + i64::from(work[3])) as i32);
        }
        if right > x && bottom > y {
            areas.push(WorkArea {
                id,
                origin: (x, y),
                logical_size: (f64::from(right - x) / scale, f64::from(bottom - y) / scale),
                scale,
            });
        }
    }
    (!areas.is_empty()).then_some((areas, foreground))
}

#[cfg(target_os = "windows")]
fn windows(fallback: &[WorkArea]) -> Option<DisplaySnapshot> {
    use std::ffi::c_void;
    #[repr(C)]
    #[derive(Default, Clone, Copy)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }
    #[repr(C)]
    struct Info {
        size: u32,
        monitor: Rect,
        work: Rect,
        flags: u32,
        name: [u16; 32],
    }
    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetForegroundWindow() -> *mut c_void;
        fn GetWindowRect(window: *mut c_void, rect: *mut Rect) -> i32;
        fn EnumDisplayMonitors(
            dc: *mut c_void,
            clip: *const Rect,
            callback: unsafe extern "system" fn(*mut c_void, *mut c_void, *mut Rect, isize) -> i32,
            data: isize,
        ) -> i32;
        fn GetMonitorInfoW(monitor: *mut c_void, info: *mut Info) -> i32;
    }
    #[link(name = "shcore")]
    unsafe extern "system" {
        fn GetDpiForMonitor(monitor: *mut c_void, kind: i32, x: *mut u32, y: *mut u32) -> i32;
    }
    unsafe extern "system" fn collect(
        monitor: *mut c_void,
        _: *mut c_void,
        _: *mut Rect,
        data: isize,
    ) -> i32 {
        let mut info = Info {
            size: std::mem::size_of::<Info>() as u32,
            monitor: Rect::default(),
            work: Rect::default(),
            flags: 0,
            name: [0; 32],
        };
        if unsafe { GetMonitorInfoW(monitor, &mut info) } != 0 {
            let mut dx = 96;
            let mut dy = 96;
            let _ = unsafe { GetDpiForMonitor(monitor, 0, &mut dx, &mut dy) };
            let scale = f64::from(dx.max(1)) / 96.;
            let r = info.work;
            let end = info.name.iter().position(|v| *v == 0).unwrap_or(32);
            let areas = unsafe { &mut *(data as *mut Vec<WorkArea>) };
            areas.push(WorkArea {
                id: String::from_utf16_lossy(&info.name[..end]),
                origin: (r.left, r.top),
                logical_size: (
                    f64::from(r.right - r.left) / scale,
                    f64::from(r.bottom - r.top) / scale,
                ),
                scale,
            });
        }
        1
    }
    let mut areas = Vec::<WorkArea>::new();
    let mut rect = Rect::default();
    let active = unsafe { GetWindowRect(GetForegroundWindow(), &mut rect) } != 0;
    unsafe {
        EnumDisplayMonitors(
            std::ptr::null_mut(),
            std::ptr::null(),
            collect,
            &mut areas as *mut _ as isize,
        )
    };
    if areas.is_empty() {
        areas = fallback.to_vec();
    }
    Some((
        areas,
        active.then_some((
            rect.left + (rect.right - rect.left) / 2,
            rect.top + (rect.bottom - rect.top) / 2,
        )),
    ))
}
