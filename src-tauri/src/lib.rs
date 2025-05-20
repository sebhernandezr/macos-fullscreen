use objc2::{rc::Retained, MainThreadMarker};
use objc2_app_kit::{NSApp, NSApplicationPresentationOptions, NSScreen, NSView, NSWindow};
use objc2_foundation::NSRect;
use std::sync::Mutex;
use tauri::{Manager, State};

pub struct Fullscreen {
    ns_window_ptr: *mut NSWindow,
    ns_view_ptr: *mut NSView,
    standard_frame: Option<NSRect>,
}

unsafe impl Send for Fullscreen {}
unsafe impl Sync for Fullscreen {}

impl Fullscreen {
    pub fn new(ns_window_ptr: *mut NSWindow, ns_view_ptr: *mut NSView) -> Self {
        Self {
            ns_window_ptr,
            ns_view_ptr,
            standard_frame: None,
        }
    }

    pub fn enter(&mut self, is_fullscreen: bool, set_frame: bool) {
        unsafe {
            let mtm = MainThreadMarker::new().unwrap();
            let ns_app = NSApp(mtm);
            let ns_screen = NSScreen::mainScreen(mtm).unwrap();
            let ns_view = Retained::retain(self.ns_view_ptr).unwrap();
            let ns_window = Retained::retain(self.ns_window_ptr).unwrap();

            let ns_window_frame = ns_window.frame();
            let frame = ns_window.contentRectForFrameRect(ns_window_frame);
            self.standard_frame = Some(frame);

            ns_app.setPresentationOptions(
                NSApplicationPresentationOptions::HideDock
                    | NSApplicationPresentationOptions::HideMenuBar
                    | NSApplicationPresentationOptions::DisableForceQuit
                    | NSApplicationPresentationOptions::DisableProcessSwitching
                    | NSApplicationPresentationOptions::DisableSessionTermination
                    | NSApplicationPresentationOptions::DisableAppleMenu
                    | NSApplicationPresentationOptions::DisableHideApplication
                    | NSApplicationPresentationOptions::DisableCursorLocationAssistance
                    | NSApplicationPresentationOptions::DisableMenuBarTransparency,
            );
            let screen_frame = ns_screen.frame();
            if set_frame {
                ns_window.setFrame_display(screen_frame, true);
            }
            if is_fullscreen {
                ns_view.enterFullScreenMode_withOptions(&ns_screen, None);
            }
        }
    }

    pub fn exit(&self, is_fullscreen: bool, set_frame: bool) {
        unsafe {
            let mtm = MainThreadMarker::new().unwrap();
            let ns_app = NSApp(mtm);
            let ns_view = Retained::retain(self.ns_view_ptr).unwrap();
            let ns_window = Retained::retain(self.ns_window_ptr).unwrap();

            ns_app.setPresentationOptions(NSApplicationPresentationOptions::Default);

            let frame = self.standard_frame.unwrap();
            if set_frame {
                ns_window.setFrame_display(frame, true);
            }
            if is_fullscreen {
                ns_view.exitFullScreenModeWithOptions(None);
            }
        }
    }

    pub fn is_fullscreen(&self) -> bool {
        unsafe {
            let ns_view = Retained::retain(self.ns_view_ptr).unwrap();
            ns_view.isInFullScreenMode()
        }
    }
}

struct AppState {
    fullscreen: Mutex<Fullscreen>,
}

#[tauri::command]
fn start(state: State<AppState>, is_fullscreen: bool, set_frame: bool) {
    let mut fullscreen = state.fullscreen.lock().unwrap();
    fullscreen.enter(is_fullscreen, set_frame);
}

#[tauri::command]
fn stop(state: State<AppState>, is_fullscreen: bool, set_frame: bool) {
    let fullscreen = state.fullscreen.lock().unwrap();
    fullscreen.exit(is_fullscreen, set_frame);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![start, stop])
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();
            let ns_window_ptr = window.ns_window().unwrap() as *mut NSWindow;
            let ns_view_ptr = window.ns_view().unwrap() as *mut NSView;
            let fullscreen = Fullscreen::new(ns_window_ptr, ns_view_ptr);
            app.manage(AppState {
                fullscreen: Mutex::new(fullscreen),
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// NOTE: static inside thread_local! implementation.
/*
use objc2::{rc::Retained, MainThreadMarker};
use objc2_app_kit::{
    NSApp, NSApplicationPresentationOptions, NSScreen, NSView, NSWindow, NSWindowSharingType,
};
use once_cell::sync::OnceCell;
use tauri::Manager;

thread_local! {
    static NS_VIEW: OnceCell<Retained<NSView>> = OnceCell::new();
    static NS_WINDOW: OnceCell<Retained<NSWindow>> = OnceCell::new();
}

pub fn enter_fullscreen(ns_window: &Retained<NSWindow>, ns_view: &Retained<NSView>) {
    let mtm = MainThreadMarker::new();
    assert!(MainThreadMarker::new().is_some());
    if let Some(mtm) = mtm {
        let ns_app = NSApp(mtm);
        ns_app.setPresentationOptions(
            NSApplicationPresentationOptions::HideDock
                | NSApplicationPresentationOptions::HideMenuBar
                | NSApplicationPresentationOptions::DisableForceQuit
                | NSApplicationPresentationOptions::DisableProcessSwitching
                | NSApplicationPresentationOptions::DisableSessionTermination
                | NSApplicationPresentationOptions::DisableAppleMenu
                | NSApplicationPresentationOptions::DisableHideApplication
                | NSApplicationPresentationOptions::DisableCursorLocationAssistance
                | NSApplicationPresentationOptions::DisableMenuBarTransparency,
        );
        let ns_screen = NSScreen::mainScreen(mtm);
        if let Some(ns_screen) = ns_screen {
            unsafe {
                ns_view.enterFullScreenMode_withOptions(&ns_screen, None);
            }
        }
        ns_window.setSharingType(NSWindowSharingType::None);
    }
}

pub fn exit_fullscreen(ns_window: &Retained<NSWindow>, ns_view: &Retained<NSView>) {
    let mtm = MainThreadMarker::new();
    assert!(MainThreadMarker::new().is_some());
    if let Some(mtm) = mtm {
        let ns_app = NSApp(mtm);
        ns_app.setPresentationOptions(NSApplicationPresentationOptions::Default);
        unsafe {
            ns_view.exitFullScreenModeWithOptions(None);
        }
        ns_window.setSharingType(NSWindowSharingType::ReadOnly);
    }
}

#[tauri::command]
fn start() {
    NS_WINDOW.with(|ns_window| {
        NS_VIEW.with(|ns_view| {
            if let (Some(ns_window), Some(ns_view)) = (ns_window.get(), ns_view.get()) {
                if unsafe { !ns_view.isInFullScreenMode() } {
                    enter_fullscreen(&ns_window, &ns_view);
                }
            }
        });
    });
}

#[tauri::command]
fn stop() {
    NS_WINDOW.with(|ns_window| {
        NS_VIEW.with(|ns_view| {
            if let (Some(ns_window), Some(ns_view)) = (ns_window.get(), ns_view.get()) {
                if unsafe { ns_view.isInFullScreenMode() } {
                    exit_fullscreen(&ns_window, &ns_view);
                }
            }
        });
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![start, stop])
        .setup(|app| {
            let webview_window = app.get_webview_window("main");
            if let Some(window) = webview_window {
                let ns_window_raw_ptr = window.ns_window().unwrap();
                let ns_window = unsafe { Retained::retain(ns_window_raw_ptr as *mut NSWindow) };
                let ns_view_raw_ptr = window.ns_view().unwrap();
                let ns_view = unsafe { Retained::retain(ns_view_raw_ptr as *mut NSView) };
                if let (Some(ns_window), Some(ns_view)) = (ns_window, ns_view) {
                    NS_WINDOW.with(|cell| {
                        let _ = cell.set(ns_window.clone());
                    });
                    NS_VIEW.with(|cell| {
                        let _ = cell.set(ns_view.clone());
                    });
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
*/

// NOTE: Objective-c implementation.
/*
@interface FullscreenWindowHelper : NSWindow<NSWindowDelegate>
- (void)goFullscreen:(NSWindow*)window
    withPresentationOptions:(NSApplicationPresentationOptions)presentationOptions
         isPresentationMode:(bool)isPresentationMode
           allowScreenShare:(bool)allowScreenShare;
- (void)releaseFullscreen;
@end

@implementation FullscreenWindowHelper {
    NSWindow* _childWindow;
    NSView* _fullscreenContentView;
    NSRect _childOriginalFrame;
}

- (void)goFullscreen:(NSWindow*)window
    withPresentationOptions:(NSApplicationPresentationOptions)presentationOptions
         isPresentationMode:(bool)isPresentationMode
           allowScreenShare:(bool)allowScreenShare {
    // Store references to the Electron window
    _childWindow = window;
    _childOriginalFrame = window.frame;
    _fullscreenContentView = [self contentView];

    // Avoid a black screen when the main window is in fullscreen during lockdown.
    if (((_childWindow.styleMask & NSFullScreenWindowMask) == NSFullScreenWindowMask)) {
        [_childWindow toggleFullScreen:nil];
    }

    // Enter fullscreen, causing the NSWindow to be replaced with _NSFullscreenWindow
    // and childing the _contentView to the _NSFullscreenWindow
    //
    // We use enterFullScreenMode since that is the method that is most reliable for blocking
    // other apps such as Alfred, see CLIENT-1313
    NSMutableDictionary* fullScreenOptions = [NSMutableDictionary dictionary];

    if (!isPresentationMode) {
        // Blacks out all external screens (and new ones that get added during lockdown).
        [fullScreenOptions setObject:[NSNumber numberWithBool:YES] forKey:NSFullScreenModeAllScreens];
    } else {
        // If we don't provide NSFullScreenModeApplicationPresentationOptions it seems that enterFullScreenMode
        // defaults to disabling all presentation options, which causes problems in presentation mode. Therefore
        // we use the same options that we used on the app level. Note that this behaviour is not documented, it
        // has been concluded by trial and error.
        [fullScreenOptions setObject:[NSNumber numberWithUnsignedInteger:presentationOptions]
                              forKey:NSFullScreenModeApplicationPresentationOptions];
    }

    [NSApp setPresentationOptions:presentationOptions];
    [_fullscreenContentView enterFullScreenMode:[NSScreen mainScreen] withOptions:fullScreenOptions];

    // Access the new _NSFullscreenWindow and append the Electron window as child
    NSWindow* fullscreenWindow = [_fullscreenContentView window];
    [fullscreenWindow addChildWindow:window ordered:NSWindowAbove];
    [fullscreenWindow setDelegate:self];

    // Ensure the fullscreen window is the key and front window
    [fullscreenWindow makeKeyAndOrderFront:nil];

    // Trigger a Window resize event with the screen size
    // to resize all content
    NSSize targetSize = [NSScreen mainScreen].frame.size;
    [self windowWillResize:fullscreenWindow toSize:targetSize];

    if (!allowScreenShare) {
        // Enable screen capture prevention. Required to be done after enterFullScreenMode.
        [fullscreenWindow setSharingType:NSWindowSharingNone];
    }
}

- (void)releaseFullscreen {
    if (_childWindow && _fullscreenContentView) {
        [_fullscreenContentView exitFullScreenModeWithOptions:nil];
        [_childWindow setParentWindow:nil];

        // Need to store instance variables in local variables
        // to avoid the dispatch to complain
        NSWindow* cw = _childWindow;
        NSRect targetFrame = _childOriginalFrame;
        _childWindow = nil;

        dispatch_after(dispatch_time(DISPATCH_TIME_NOW, 0 * NSEC_PER_SEC), dispatch_get_main_queue(), ^{
          // orderfront brings the window to the front, but don't work if called directly.
          // for some reason it works properly if called after the current call stack
          [cw orderFront:nil];
          [cw setFrame:targetFrame display:YES];
        });
    }

    NSApplicationPresentationOptions presentationOptions = NSApplicationPresentationDefault;
    [NSApp setPresentationOptions:presentationOptions];
}

// Event listener to change the child window size if the size of the fullscreen window changes
- (NSSize)windowWillResize:(NSWindow*)sender toSize:(NSSize)frameSize {
    if (_childWindow) {
        [_childWindow setFrame:NSMakeRect(0, 0, frameSize.width, frameSize.height) display:YES];
    }

    return frameSize;
}
@end
*/
