mod configuration;
mod consts;
mod handlers;
mod helpers;
mod log;
mod sys;
mod takure;
mod types;
mod updater;

use std::thread;

#[cfg(feature = "autoupdate")]
use std::sync::{mpsc, Mutex, OnceLock};

#[cfg(feature = "autoupdate")]
use crate::takure::hook_init_from_cached_info;
use crate::helpers::{LibraryHandle, ThreadHandle};
use crate::log::Logger;
use crate::takure::{hook_init, hook_release};
use ::log::{error, info};
use configuration::Configuration;
use lazy_static::lazy_static;
use url::Url;
use winapi::shared::minwindef::{BOOL, DWORD, HINSTANCE, LPVOID, TRUE};
use winapi::um::consoleapi::AllocConsole;
use winapi::um::winnt::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH};

lazy_static! {
    pub static ref CONFIGURATION: Configuration = {
        let result = Configuration::load();
        if let Err(err) = result {
            error!("{:#}", err);
            std::process::exit(1);
        }

        result.unwrap()
    };

    pub static ref TACHI_STATUS_URL: String = {
        let result = Url::parse(&CONFIGURATION.tachi.base_url)
            .and_then(|url| url.join("/api/v1/status"));
        if let Err(err) = result {
            error!("Could not parse Tachi status URL: {:#}", err);
            std::process::exit(1);
        }

        result.unwrap().to_string()
    };
    pub static ref TACHI_IMPORT_URL: String = {
        let result = Url::parse(&CONFIGURATION.tachi.base_url)
            .and_then(|url| url.join("/ir/direct-manual/import"));
        if let Err(err) = result {
            error!("Could not parse Tachi import URL: {:#}", err);
            std::process::exit(1);
        }

        result.unwrap().to_string()
    };
}

fn init_logger() {
    env_logger::builder()
        .filter_level(::log::LevelFilter::Error)
        .filter_module(
            "takure",
            if cfg!(debug_assertions) {
                ::log::LevelFilter::Debug
            } else {
                ::log::LevelFilter::Info
            },
        )
        .parse_default_env()
        .target(env_logger::Target::Pipe(Box::new(Logger::new())))
        .format(|f, record| {
            use crate::log::{colored_level, max_target_width, Padded};
            use std::io::Write;

            let target = record.target();
            let max_width = max_target_width(target);

            let mut style = f.style();
            let level = colored_level(&mut style, record.level());

            let mut style = f.style();
            let target = style.set_bold(true).value(Padded {
                value: target,
                width: max_width,
            });

            let time = chrono::Local::now().format("%d/%m/%Y %H:%M:%S");

            writeln!(f, "[{}] {} {} -> {}", time, level, target, record.args())
        })
        .init();
}

fn print_infos() {
    info!(
        "Starting Takure v{}-{} by auxbh",
        env!("CARGO_PKG_VERSION"),
        option_env!("VERGEN_GIT_DESCRIBE").unwrap_or("unknown")
    );

    if let Some(build_date) = option_env!("VERGEN_BUILD_DATE") {
        info!("Build date: {}", build_date);
    }
}

// Set right before crochet::enable!(avs_ea3_boot_startup_hook) in DllMain, so it's guaranteed
// present by the time AVS could possibly reach the boot hook. Lets the boot hook wake the
// worker thread waiting to self-update only *after* call_original! has run — self-updating
// needs unbounded network time, so it must never run before the hook is installed (it would
// race AVS's one-shot boot call) nor from the boot thread itself (killing that thread, even
// after call_original! returns, takes down whatever AVS still needs it for).
#[cfg(feature = "autoupdate")]
static BOOT_SIGNAL_TX: OnceLock<Mutex<mpsc::Sender<()>>> = OnceLock::new();

#[cfg_attr(target_arch = "x86", crochet::hook("libavs-win32-ea3.dll", "XE592acd00008c"))]
#[cfg_attr(target_arch = "x86_64", crochet::hook("libavs-win64-ea3.dll", "XEyy2igh000007"))]
unsafe extern "C" fn avs_ea3_boot_startup_hook(node: *const ()) -> i32 {
    if let Err(err) = hook_init(node) {
        error!("{:#}", err);
    }

    let result = call_original!(node);

    #[cfg(feature = "autoupdate")]
    if let Some(tx) = BOOT_SIGNAL_TX.get() {
        if let Ok(tx) = tx.lock() {
            let _ = tx.send(());
        }
    }

    result
}

#[no_mangle]
#[allow(non_snake_case, unused_variables)]
extern "system" fn DllMain(dll_module: HINSTANCE, call_reason: DWORD, reserved: LPVOID) -> BOOL {
    match call_reason {
        DLL_PROCESS_ATTACH => {
            unsafe { AllocConsole() };
            init_logger();

            // A self-update reload sets this before exiting; AVS's boot event won't fire
            // again for the reloaded module, so this instance skips waiting for it.
            #[cfg(feature = "autoupdate")]
            let reloaded_after_update = std::env::var(consts::GAME_INFO_ENV).is_ok();
            #[cfg(not(feature = "autoupdate"))]
            let reloaded_after_update = false;

            #[cfg(feature = "autoupdate")]
            let boot_rx = if !reloaded_after_update {
                let (tx, rx) = mpsc::channel();
                let _ = BOOT_SIGNAL_TX.set(Mutex::new(tx));
                Some(rx)
            } else {
                None
            };

            if !reloaded_after_update {
                if let Err(err) = crochet::enable!(avs_ea3_boot_startup_hook) {
                    error!("{:#}", err);
                }
            }

            #[cfg_attr(not(feature = "autoupdate"), allow(unused_variables))]
            let library_handle = unsafe { LibraryHandle::new(dll_module) };
            let thread_handle = ThreadHandle::duplicate_current_thread_handle();

            thread::spawn(move || {
                // Wait until DllMain returns and the loader lock is released before doing
                // any real work, otherwise self-updating (which needs to spawn its own
                // thread and touch the filesystem) can deadlock. The boot hook is already
                // installed by this point regardless, so this wait can't cause a missed boot.
                if let Ok(h) = thread_handle {
                    h.wait_and_close(1000);
                }

                print_infos();

                #[cfg(feature = "autoupdate")]
                if reloaded_after_update {
                    if let Err(err) = hook_init_from_cached_info() {
                        error!("{:#}", err);
                    }
                    return;
                }

                #[cfg(feature = "autoupdate")]
                if let Some(rx) = boot_rx {
                    // Blocks until the boot hook has captured game info, installed
                    // property_destroy_hook, and let call_original! run to completion.
                    let _ = rx.recv();

                    if CONFIGURATION.general.auto_update {
                        match updater::self_update(&library_handle) {
                            Ok(true) => {
                                info!("Self-update successful. Reloading into new hook...");
                                library_handle.free_and_exit_thread(1);
                            }
                            Ok(false) => {}
                            Err(e) => {
                                error!("Self-update failed: {e:#}");
                            }
                        }
                    }
                }
            });
        }
        DLL_PROCESS_DETACH => {
            if crochet::is_enabled!(avs_ea3_boot_startup_hook) {
                if let Err(err) = crochet::disable!(avs_ea3_boot_startup_hook) {
                    error!("{:#}", err);
                }
            }

            if let Err(err) = hook_release() {
                error!("{:#}", err);
            }
        }
        _ => {}
    }

    TRUE
}
