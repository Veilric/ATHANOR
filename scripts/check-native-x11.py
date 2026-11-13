#!/usr/bin/env python3
"""Controlled X11/XTest smoke checks against an already-running Athanor instance.
Creates its own input sentinel. Never types into unrelated windows. Restores focus/pointer.
Requires wmctrl, xprop, libX11 and libXtst. Fails rather than assuming native behavior.
"""
import argparse
import ctypes as c
import ctypes.util
import json
from pathlib import Path
import statistics
import subprocess
import time

p = argparse.ArgumentParser()
p.add_argument("--output", type=Path, default=Path("artifacts/native-x11.json"))
p.add_argument("--leave",choices=["hidden","compact","expanded"],default="hidden")
p.add_argument("--set-state",choices=["hidden","compact","expanded"],help="Only prepare an already-pinned test instance for a resource sample")
a = p.parse_args()
x = c.CDLL(ctypes.util.find_library("X11"))
t = c.CDLL(ctypes.util.find_library("Xtst"))
Display = c.c_void_p
Window = c.c_ulong
def bind(lib, name, restype, *args):
    fn = getattr(lib, name); fn.restype = restype; fn.argtypes = list(args); return fn
bind(x,"XOpenDisplay",Display,c.c_char_p)
bind(x,"XDefaultRootWindow",Window,Display)
bind(x,"XStringToKeysym",c.c_ulong,c.c_char_p)
bind(x,"XKeysymToKeycode",c.c_uint,Display,c.c_ulong)
bind(x,"XGetInputFocus",c.c_int,Display,c.POINTER(Window),c.POINTER(c.c_int))
bind(x,"XQueryPointer",c.c_int,Display,Window,c.POINTER(Window),c.POINTER(Window),c.POINTER(c.c_int),c.POINTER(c.c_int),c.POINTER(c.c_int),c.POINTER(c.c_int),c.POINTER(c.c_uint))
bind(x,"XSetInputFocus",c.c_int,Display,Window,c.c_int,c.c_ulong)
bind(x,"XCreateSimpleWindow",Window,Display,Window,c.c_int,c.c_int,c.c_uint,c.c_uint,c.c_uint,c.c_ulong,c.c_ulong)
bind(x,"XStoreName",c.c_int,Display,Window,c.c_char_p)
bind(x,"XSelectInput",c.c_int,Display,Window,c.c_long)
bind(x,"XMapWindow",c.c_int,Display,Window)
bind(x,"XDestroyWindow",c.c_int,Display,Window)
bind(x,"XFlush",c.c_int,Display)
bind(x,"XInternAtom",c.c_ulong,Display,c.c_char_p,c.c_int)
bind(x,"XConvertSelection",c.c_int,Display,c.c_ulong,c.c_ulong,c.c_ulong,Window,c.c_ulong)
bind(x,"XGetWindowProperty",c.c_int,Display,Window,c.c_ulong,c.c_long,c.c_long,c.c_int,c.c_ulong,c.POINTER(c.c_ulong),c.POINTER(c.c_int),c.POINTER(c.c_ulong),c.POINTER(c.c_ulong),c.POINTER(c.POINTER(c.c_ubyte)))
bind(x,"XFree",c.c_int,c.c_void_p)
bind(x,"XPending",c.c_int,Display)
bind(x,"XNextEvent",c.c_int,Display,c.c_void_p)
bind(x,"XGetGeometry",c.c_int,Display,Window,c.POINTER(Window),c.POINTER(c.c_int),c.POINTER(c.c_int),c.POINTER(c.c_uint),c.POINTER(c.c_uint),c.POINTER(c.c_uint),c.POINTER(c.c_uint))
bind(x,"XTranslateCoordinates",c.c_int,Display,Window,Window,c.c_int,c.c_int,c.POINTER(c.c_int),c.POINTER(c.c_int),c.POINTER(Window))
bind(t,"XTestFakeKeyEvent",c.c_int,Display,c.c_uint,c.c_int,c.c_ulong)
bind(t,"XTestFakeMotionEvent",c.c_int,Display,c.c_int,c.c_int,c.c_int,c.c_ulong)
bind(t,"XTestFakeButtonEvent",c.c_int,Display,c.c_uint,c.c_int,c.c_ulong)
d=x.XOpenDisplay(None)
assert d,"No X11 display"
root=x.XDefaultRootWindow(d)
previous=Window(); revert=c.c_int(); x.XGetInputFocus(d,c.byref(previous),c.byref(revert))
pointer_root=Window();pointer_child=Window();pointer_x=c.c_int();pointer_y=c.c_int();window_x=c.c_int();window_y=c.c_int();pointer_mask=c.c_uint()
x.XQueryPointer(d,root,c.byref(pointer_root),c.byref(pointer_child),c.byref(pointer_x),c.byref(pointer_y),c.byref(window_x),c.byref(window_y),c.byref(pointer_mask))

def focus():
    value=Window(); hint=c.c_int();x.XGetInputFocus(d,c.byref(value),c.byref(hint));return value.value
def key(name, down):
    code=x.XKeysymToKeycode(d,x.XStringToKeysym(name.encode()));assert code,name
    t.XTestFakeKeyEvent(d,code,down,0);x.XFlush(d)
def press(name):key(name,1);key(name,0)
def shortcut():
    key("Control_L",1);key("Alt_L",1);press("space");key("Alt_L",0);key("Control_L",0)
def wait(fn, timeout=4):
    until=time.monotonic()+timeout
    while not fn():
        assert time.monotonic()<until,"Native condition timed out"
        time.sleep(.005)
def windows():return subprocess.check_output(["wmctrl","-lp"],text=True).splitlines()
def panel():
    rows=[line for line in windows() if line.endswith(" Athanor")]
    return int(rows[0].split()[0],16) if rows else None
shortcut()
wait(panel,20)
w=panel()
def geometry():
    r=Window();xx=c.c_int();yy=c.c_int();width=c.c_uint();height=c.c_uint();border=c.c_uint();depth=c.c_uint()
    x.XGetGeometry(d,w,c.byref(r),c.byref(xx),c.byref(yy),c.byref(width),c.byref(height),c.byref(border),c.byref(depth))
    child=Window();x.XTranslateCoordinates(d,w,root,0,0,c.byref(xx),c.byref(yy),c.byref(child))
    return xx.value,yy.value,width.value,height.value
def visible():
    # WM_STATE is Iconic/Withdrawn when hidden; query the server's mapped window list.
    state=subprocess.check_output(["xprop","-root","_NET_CLIENT_LIST"],text=True)
    return any(int(word.strip(","),16)==w for word in state.split() if word.startswith("0x"))
def click(px,py):
    xx,yy,_,_=geometry();t.XTestFakeMotionEvent(d,-1,xx+px,yy+py,0)
    t.XTestFakeButtonEvent(d,1,1,0);t.XTestFakeButtonEvent(d,1,0,0);x.XFlush(d)

def on_top():
    state = subprocess.check_output(["xprop", "-id", hex(w), "_NET_WM_STATE"], text=True)
    return "_NET_WM_STATE_ABOVE" in state

def drag_handle():
    start_x, start_y, _, _ = geometry()
    pointer_x, pointer_y = start_x + 22, start_y + 26
    t.XTestFakeMotionEvent(d, -1, pointer_x, pointer_y, 0)
    t.XTestFakeButtonEvent(d, 1, 1, 0)
    x.XFlush(d)
    time.sleep(.1)
    for step in range(1, 7):
        t.XTestFakeMotionEvent(d, -1, pointer_x - step * 12, pointer_y + step * 8, 0)
        x.XFlush(d)
        time.sleep(.04)
    t.XTestFakeButtonEvent(d, 1, 0, 0)
    x.XFlush(d)
    wait(lambda: geometry()[:2] != (start_x, start_y))
    assert visible(), "Dragging must preserve visibility"

def focus_sentinel():
    child = Window()
    target_x, target_y = c.c_int(), c.c_int()
    x.XTranslateCoordinates(d, sentinel, root, 100, 60, c.byref(target_x), c.byref(target_y), c.byref(child))
    t.XTestFakeMotionEvent(d, -1, target_x.value, target_y.value, 0)
    x.XFlush(d)
    subprocess.run(["wmctrl", "-i", "-a", hex(sentinel)], check=True)
    wait(lambda: focus() == sentinel)

if a.set_state:
    shortcut();wait(lambda:focus()==w);time.sleep(.15)
    assert focus()==w
    if a.set_state=="hidden":
        press("Escape");wait(lambda:focus()!=w)
    else:
        key("Control_L",1);press("a");key("Control_L",0);press("BackSpace")
        if a.set_state=="expanded":
            press("2");key("Shift_L",1);press("equal");key("Shift_L",0);press("2");wait(lambda:geometry()[3]==238)
        else:wait(lambda:geometry()[3]==53,5)
        if previous.value not in [0,w]:x.XSetInputFocus(d,previous,1,0);x.XFlush(d)
    print(a.set_state)
    raise SystemExit(0)

sentinel=x.XCreateSimpleWindow(d,root,40,80,350,120,0,0,0xE8EEF2)
x.XStoreName(d,sentinel,b"Athanor controlled input sentinel")
x.XSelectInput(d,sentinel,1);x.XFlush(d)
results={}
try:
    # Settle the first reveal before the assertions; map the sentinel only once pinned.
    time.sleep(.3)
    shortcut()
    wait(lambda:focus()==w)
    time.sleep(.15)
    assert focus()==w
    wait(lambda: not on_top())
    results["unpinned_normal_window_level"] = True
    drag_handle()
    results["unpinned_drag_moves_window"] = True
    time.sleep(1.1)
    click(22, 26)
    assert visible(), "Clicking the unpinned handle must preserve visibility"
    unpinned_position = geometry()[:2]
    results["unpinned_drag_and_handle_preserve_visibility"] = True
    x.XMapWindow(d, sentinel)
    x.XFlush(d)
    time.sleep(.15)
    focus_sentinel()
    assert visible() and not on_top()
    shortcut()
    wait(lambda: focus() == w)
    assert geometry()[:2] == unpinned_position
    results["unpinned_focus_loss_visible_normal_level_position_retained"] = True
    # Empty pinned composer must collapse to 53px and keep its input anchored.
    before=geometry();click(270,26)
    wait(on_top)
    wait(lambda:geometry()[3]==53,5)
    compact=geometry();assert compact[:2]==before[:2]
    results["pinned_empty_idle_53px"]=True
    results["pin_preserves_position_and_enables_topmost"] = True
    drag_handle()
    compact = geometry()
    assert on_top()
    results["pinned_drag_preserves_visibility_and_topmost"] = True
    focus_sentinel()
    while x.XPending(d):
        event=(c.c_long*24)();x.XNextEvent(d,c.byref(event))
    press("a");time.sleep(.15)
    received=False
    while x.XPending(d):
        event=(c.c_long*24)();x.XNextEvent(d,c.byref(event))
        received |= c.cast(c.byref(event),c.POINTER(c.c_int))[0]==2
    assert received and geometry()[3]==53 and on_top()
    results["outside_typing_untouched"]=True
    shortcut();wait(lambda:focus()==w)
    press("2");key("Shift_L",1);press("equal");key("Shift_L",0);press("2")
    wait(lambda:geometry()[3]==238)
    assert geometry()[:2]==compact[:2]
    time.sleep(3.1);assert geometry()[3]==238
    results["typing_expands_and_nonempty_survives_idle"]=True
    press("Return");time.sleep(.15)
    selection=x.XInternAtom(d,b"CLIPBOARD",0);utf8=x.XInternAtom(d,b"UTF8_STRING",0);property_atom=x.XInternAtom(d,b"ATHANOR_TEST_SELECTION",0)
    x.XConvertSelection(d,selection,utf8,property_atom,sentinel,0);x.XFlush(d)
    def clipboard_matches():
        actual=c.c_ulong();fmt=c.c_int();length=c.c_ulong();remaining=c.c_ulong();data=c.POINTER(c.c_ubyte)()
        x.XGetWindowProperty(d,sentinel,property_atom,0,64,0,utf8,c.byref(actual),c.byref(fmt),c.byref(length),c.byref(remaining),c.byref(data))
        try:return bool(data) and c.string_at(data,length.value)==b"4"
        finally:
            if data:x.XFree(data)
    wait(clipboard_matches)
    results["calculator_copy_uses_native_clipboard"]=True
    press("Escape");wait(lambda:focus()!=w)
    results["escape_yields_focus"]=True
    # The restored panel is empty (197px) rather than the previous answer (238px).
    shortcut();wait(lambda:focus()==w);wait(lambda:geometry()[3]==197)
    # Unpin; subsequent invocations don't wait for idle.
    pinned_position = geometry()[:2]
    click(270,26)
    wait(lambda: not on_top())
    assert geometry()[:2] == pinned_position
    click(100,26)
    press("2");key("Shift_L",1);press("equal");key("Shift_L",0);press("2")
    wait(lambda: geometry()[3] == 238)
    press("Return")
    time.sleep(.15)
    assert visible()
    results["unpin_preserves_position_and_successful_action_stays_visible"] = True
    press("Escape");wait(lambda:focus()!=w)
    timings=[]
    for _ in range(30):
        start=time.perf_counter();shortcut();wait(lambda:focus()==w)
        timings.append((time.perf_counter()-start)*1000)
        press("Escape");wait(lambda:focus()!=w);time.sleep(.04)
    results["escape_clears_query"]=True
    results["shortcut_30_invocations"]=True
    results["warm_focus_p95_ms"]=sorted(timings)[28]
    results["warm_focus_median_ms"]=statistics.median(timings)
    results["timing_definition"]="XTest shortcut to X input focus; includes 5ms polling; not first presented pixels"
    shortcut()
    wait(lambda: focus() == w)
    subprocess.run(["wmctrl", "-i", "-c", hex(w)], check=True)
    wait(lambda: focus() != w)
    wait(lambda: panel() is None)
    shortcut()
    wait(panel)
    w = panel()
    wait(lambda: focus() == w)
    wait(lambda: geometry()[3] == 197)
    assert not on_top()
    results["explicit_close_and_summon_remain_functional"] = True
    press("Escape")
    wait(lambda: focus() != w)
    if a.leave!="hidden":
        shortcut();wait(lambda:focus()==w);click(270,26)
        wait(lambda:geometry()[3]==53,5)
        if a.leave=="expanded":
            press("2");key("Shift_L",1);press("equal");key("Shift_L",0);press("2");wait(lambda:geometry()[3]==238)
    results["left_state"]=a.leave
except Exception:
    results["failure_focus_window"] = hex(focus())
    results["failure_panel_window"] = hex(w)
    results["failure_sentinel_window"] = hex(sentinel)
    results["failure_panel_properties"] = subprocess.run(
        ["xprop", "-id", hex(w), "WM_STATE", "_NET_WM_STATE"],
        capture_output=True, text=True, check=False,
    ).stdout
    raise
finally:
    a.output.parent.mkdir(parents=True,exist_ok=True)
    a.output.write_text(json.dumps(results,indent=2)+"\n")
    for modifier in ["Control_L","Alt_L","Shift_L"]:key(modifier,0)
    t.XTestFakeButtonEvent(d,1,0,0)
    t.XTestFakeMotionEvent(d,-1,pointer_x.value,pointer_y.value,0)
    if previous.value not in [0,1,w,sentinel]:
        subprocess.run(["wmctrl","-i","-a",hex(previous.value)],check=False)
    x.XDestroyWindow(d,sentinel);x.XFlush(d)
print(json.dumps(results,indent=2))
