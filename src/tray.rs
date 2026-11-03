use std::ffi::{c_char, c_int, c_uint, c_void, CString};
use std::ptr;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use eframe::egui;

#[derive(Clone, Copy)]
pub enum TrayAction {
    ShowWindow,
    HideWindow,
    ToggleProcessing,
    Quit,
}

pub enum TrayEvent {
    Action(TrayAction),
    Unavailable(String),
}

struct CallbackData {
    sender: Sender<TrayEvent>,
    context: egui::Context,
    action: TrayAction,
}

#[link(name = "ayatana-appindicator3")]
#[link(name = "gtk-3")]
#[link(name = "gobject-2.0")]
unsafe extern "C" {
    fn app_indicator_new_with_path(
        id: *const c_char,
        icon_name: *const c_char,
        category: c_int,
        icon_theme_path: *const c_char,
    ) -> *mut c_void;
    fn app_indicator_set_status(indicator: *mut c_void, status: c_int);
    fn app_indicator_set_title(indicator: *mut c_void, title: *const c_char);
    fn app_indicator_set_menu(indicator: *mut c_void, menu: *mut c_void);
    fn app_indicator_set_secondary_activate_target(indicator: *mut c_void, menu_item: *mut c_void);

    fn gtk_init_check(argc: *mut c_int, argv: *mut *mut *mut c_char) -> c_int;
    fn gtk_main();
    fn gtk_menu_new() -> *mut c_void;
    fn gtk_menu_item_new_with_label(label: *const c_char) -> *mut c_void;
    fn gtk_separator_menu_item_new() -> *mut c_void;
    fn gtk_menu_shell_append(menu_shell: *mut c_void, child: *mut c_void);
    fn gtk_widget_show_all(widget: *mut c_void);

    fn g_signal_connect_data(
        instance: *mut c_void,
        signal: *const c_char,
        callback: Option<unsafe extern "C" fn()>,
        data: *mut c_void,
        destroy_data: Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
        connect_flags: c_uint,
    ) -> u64;
}

type MenuCallback = unsafe extern "C" fn(*mut c_void, *mut c_void);

pub fn start(context: egui::Context, spanish: bool) -> Receiver<TrayEvent> {
    let (sender, receiver) = mpsc::channel();
    let thread_sender = sender.clone();

    if let Err(error) = thread::Builder::new()
        .name("eqlinux-indicator".into())
        .spawn(move || {
            let initialized = unsafe { gtk_init_check(ptr::null_mut(), ptr::null_mut()) };
            if initialized == 0 {
                let _ = thread_sender.send(TrayEvent::Unavailable(
                    "GTK could not connect to the desktop session".into(),
                ));
                return;
            }

            unsafe {
                let menu = gtk_menu_new();
                if menu.is_null() {
                    let _ = thread_sender.send(TrayEvent::Unavailable(
                        "Could not create indicator menu".into(),
                    ));
                    return;
                }

                let show_item = add_menu_item(
                    menu,
                    if spanish {
                        "Mostrar eqLinux"
                    } else {
                        "Show eqLinux"
                    },
                    TrayAction::ShowWindow,
                    &thread_sender,
                    &context,
                );
                add_menu_item(
                    menu,
                    if spanish {
                        "Ocultar eqLinux"
                    } else {
                        "Hide eqLinux"
                    },
                    TrayAction::HideWindow,
                    &thread_sender,
                    &context,
                );
                add_menu_item(
                    menu,
                    if spanish {
                        "Activar/desactivar procesamiento de audio"
                    } else {
                        "Enable / disable audio processing"
                    },
                    TrayAction::ToggleProcessing,
                    &thread_sender,
                    &context,
                );
                let sep = gtk_separator_menu_item_new();
                if !sep.is_null() {
                    gtk_menu_shell_append(menu, sep);
                }
                add_menu_item(
                    menu,
                    if spanish {
                        "Salir de eqLinux"
                    } else {
                        "Quit eqLinux"
                    },
                    TrayAction::Quit,
                    &thread_sender,
                    &context,
                );
                gtk_widget_show_all(menu);

                let id = CString::new("eqlinux").expect("static string");
                let icon = CString::new("eqlinux").expect("static string");
                let title = CString::new("eqLinux").expect("static string");
                let icon_path = dirs::data_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("."))
                    .join("icons/hicolor/48x48/apps");
                let icon_path = CString::new(icon_path.to_string_lossy().as_bytes())
                    .expect("icon path cannot contain a NUL byte");
                let indicator =
                    app_indicator_new_with_path(id.as_ptr(), icon.as_ptr(), 0, icon_path.as_ptr());
                if indicator.is_null() {
                    let _ = thread_sender.send(TrayEvent::Unavailable(
                        "Could not create the desktop indicator".into(),
                    ));
                    return;
                }
                app_indicator_set_title(indicator, title.as_ptr());
                app_indicator_set_menu(indicator, menu);
                if !show_item.is_null() {
                    app_indicator_set_secondary_activate_target(indicator, show_item);
                }
                app_indicator_set_status(indicator, 1);
                gtk_main();
            }
        })
    {
        let _ = sender.send(TrayEvent::Unavailable(format!(
            "Could not start indicator thread: {error}"
        )));
    }

    receiver
}

unsafe fn add_menu_item(
    menu: *mut c_void,
    label: &str,
    action: TrayAction,
    sender: &Sender<TrayEvent>,
    context: &egui::Context,
) -> *mut c_void {
    let label = CString::new(label).expect("static label");
    let item = gtk_menu_item_new_with_label(label.as_ptr());
    if item.is_null() {
        return ptr::null_mut();
    }
    gtk_menu_shell_append(menu, item);

    let callback_data = Box::new(CallbackData {
        sender: sender.clone(),
        context: context.clone(),
        action,
    });
    let signal = CString::new("activate").expect("static signal");
    let callback: MenuCallback = on_menu_activate;
    let callback = Some(std::mem::transmute::<MenuCallback, unsafe extern "C" fn()>(
        callback,
    ));
    g_signal_connect_data(
        item,
        signal.as_ptr(),
        callback,
        Box::into_raw(callback_data).cast(),
        Some(drop_callback_data),
        0,
    );
    item
}

unsafe extern "C" fn on_menu_activate(_item: *mut c_void, data: *mut c_void) {
    let Some(data) = data.cast::<CallbackData>().as_ref() else {
        return;
    };
    if data.sender.send(TrayEvent::Action(data.action)).is_ok() {
        data.context.request_repaint();
    }
}

unsafe extern "C" fn drop_callback_data(data: *mut c_void, _closure: *mut c_void) {
    if !data.is_null() {
        drop(Box::from_raw(data.cast::<CallbackData>()));
    }
}
