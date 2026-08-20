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
#[cfg(feature = "autoupdate")]
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
    let describe = option_env!("VERGEN_GIT_DESCRIBE").unwrap_or("unknown");
    if describe.starts_with('v') {
        info!("Starting Takure {} by auxbh", describe);
    } else {
        info!("Starting Takure v{}-{} by auxbh", env!("CARGO_PKG_VERSION"), describe);
    }

    if let Some(build_date) = option_env!("VERGEN_BUILD_DATE") {
        info!("Build date: {}", build_date);
    }
}

// Notify-only fallback used when self-update isn't running (feature off or auto_update disabled)
fn check_for_update() -> anyhow::Result<()> {
    let describe = option_env!("VERGEN_GIT_DESCRIBE").unwrap_or("unknown");
    if describe.contains('-') {
        return Ok(());
    }

    let latest_tag = helpers::request_agent()
        .get("https://api.github.com/repos/auxbh/takure/releases/latest")
        .call()?
        .into_json::<serde_json::Value>()?
        .get("tag_name")
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
        .ok_or(anyhow::anyhow!("Could not get latest release tag"))?;

    if describe != latest_tag && !cfg!(debug_assertions) {
        info!("A newer version of Takure is available at https://github.com/auxbh/takure/releases/latest");
    }

    Ok(())
}

// Signals the worker thread once call_original! has run, so network I/O never races AVS's
// one-shot boot call and never blocks the boot thread itself
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
            print_infos();

            // Set by a self-update reload; the boot event won't fire again this session
            #[cfg(feature = "autoupdate")]
            let reloaded_after_update = std::env::var(consts::GAME_INFO_ENV).is_ok();
            #[cfg(not(feature = "autoupdate"))]
            let reloaded_after_update = false;

            #[cfg(feature = "autoupdate")]
            let self_update_enabled = !reloaded_after_update && CONFIGURATION.general.auto_update;
            #[cfg(not(feature = "autoupdate"))]
            let self_update_enabled = false;

            if !reloaded_after_update && !self_update_enabled {
                thread::spawn(|| {
                    if let Err(err) = check_for_update() {
                        error!("Unable to get update informations {:#}", err);
                    }
                });
            }

            #[cfg(feature = "autoupdate")]
            let boot_rx = if self_update_enabled {
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

            #[cfg(feature = "autoupdate")]
            {
                let library_handle = unsafe { LibraryHandle::new(dll_module) };
                let thread_handle = ThreadHandle::duplicate_current_thread_handle();

                thread::spawn(move || {
                    // Avoid deadlock: wait for DllMain to release the loader lock first
                    if let Ok(h) = thread_handle {
                        h.wait_and_close(1000);
                    }

                    if reloaded_after_update {
                        if let Err(err) = hook_init_from_cached_info() {
                            error!("{:#}", err);
                        }
                        return;
                    }

                    if let Some(rx) = boot_rx {
                        let _ = rx.recv();

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
                });
            }
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
