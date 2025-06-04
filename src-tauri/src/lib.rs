use objc2::rc::Retained;
use objc2::{MainThreadMarker, Message};
use objc2_app_kit::{
    NSApp, NSApplicationPresentationOptions, NSScreen, NSView, NSWindow, NSWindowOrderingMode,
};
use objc2_foundation::NSRect;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};

#[derive(Debug, Default)]
pub struct Fullscreen {
    state: FullscreenState,
}

#[derive(Debug, Default)]
enum FullscreenState {
    #[default]
    Normal,
    Fullscreen {
        child_window: Retained<NSWindow>,
        fullscreen_content_view: Retained<NSView>,
        original_frame: NSRect,
    },
}

/// SAFETY: Each method must ensure that they are only accessing values on the main thread.
unsafe impl Send for Fullscreen {}
/// SAFETY: Each method must ensure that they are only accessing values on the main thread.
unsafe impl Sync for Fullscreen {}

impl Fullscreen {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn enter(&mut self, app_handle: &AppHandle) {
        // Ensure we're running on the main thread.
        let mtm = MainThreadMarker::new().unwrap();
        let ns_app = NSApp(mtm);

        let window = app_handle.get_webview_window("main").unwrap();
        let ns_window = unsafe { window.ns_window().unwrap().cast::<NSWindow>().as_ref() }
            .unwrap()
            .retain();
        let ns_view = unsafe { window.ns_view().unwrap().cast::<NSView>().as_ref() }
            .unwrap()
            .retain();

        let ns_screen = NSScreen::mainScreen(mtm).unwrap();

        // Get the size of the window before resizing.
        let original_frame = ns_window.contentRectForFrameRect(ns_window.frame());

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

        // Maximize webview window.
        let screen_frame = ns_screen.frame();
        ns_view
            .window()
            .unwrap()
            .setFrame_display(screen_frame, true);

        // Enter fullscreen.
        unsafe { ns_view.enterFullScreenMode_withOptions(&ns_screen, None) };
        // Get the new window, the fullscreen window.
        let fullscreen_window = ns_view.window().unwrap();

        // Add the webview window as a child of the fullscreen window.
        unsafe {
            fullscreen_window.addChildWindow_ordered(&ns_window, NSWindowOrderingMode::Below)
        };

        // Ensure the fullscreen window can handle *some* events.
        let delegate = unsafe { ns_window.delegate() }.unwrap();
        fullscreen_window.setDelegate(Some(&delegate));

        // Display fullscreen window and give it focus.
        fullscreen_window.makeKeyAndOrderFront(None);

        self.state = FullscreenState::Fullscreen {
            child_window: ns_window,
            fullscreen_content_view: ns_view,
            original_frame,
        };
    }

    pub fn exit(&mut self) {
        // Ensure we're running on the main thread.
        let mtm = MainThreadMarker::new().unwrap();
        let ns_app = NSApp(mtm);

        let FullscreenState::Fullscreen {
            child_window,
            fullscreen_content_view,
            original_frame,
        } = std::mem::take(&mut self.state)
        else {
            eprintln!("Not in fullscreen mode");
            return;
        };

        unsafe {
            fullscreen_content_view.exitFullScreenModeWithOptions(None);
            child_window.setParentWindow(None);
        }

        // Display original window again and make it focused.
        child_window.makeKeyAndOrderFront(None);
        // Restore window size.
        child_window.setFrame_display(original_frame, true);

        ns_app.setPresentationOptions(NSApplicationPresentationOptions::Default);
    }

    pub fn is_fullscreen(&self) -> bool {
        match &self.state {
            FullscreenState::Fullscreen {
                fullscreen_content_view,
                ..
            } => {
                // SAFETY: I think this function should be ok to call from any thread.
                unsafe { fullscreen_content_view.isInFullScreenMode() }
            },
            _ => false,
        }
    }
}

struct AppState {
    fullscreen: Mutex<Fullscreen>,
}

#[tauri::command]
fn start(app_handle: AppHandle, state: State<AppState>) {
    let mut fullscreen = state.fullscreen.lock().unwrap();
    fullscreen.enter(&app_handle);
}

#[tauri::command]
fn stop(state: State<AppState>) {
    let mut fullscreen = state.fullscreen.lock().unwrap();
    fullscreen.exit();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![start, stop])
        .setup(|app| {
            let fullscreen = Fullscreen::new();
            app.manage(AppState {
                fullscreen: Mutex::new(fullscreen),
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

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
