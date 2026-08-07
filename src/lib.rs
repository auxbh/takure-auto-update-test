mod configuration;
mod consts;
mod handlers;
mod helpers;
mod log;
mod sys;
mod takure;
mod types;
mod updater;

#[cfg(feature = "autoupdate")]
use std::thread;

#[cfg(feature = "autoupdate")]
use crate::helpers::{LibraryHandle, ThreadHandle};
use crate::log::Logger;
use crate::takure::{hook_init, hook_release};
use ::log::{error, info};
use configuration::Configuration;
use lazy_static::lazy_static;
#[cfg(feature = "autoupdate")]
use std::sync::OnceLock;
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

// Captured in DllMain, on DLL_PROCESS_ATTACH, before AVS can possibly reach the boot hook
// below (boot cannot fire until DllMain returns). Self-update needs its own module handle
// to locate the DLL on disk and to free/reload itself, but the boot hook receives no such
// handle from AVS, so it's stashed here instead of threading it through call_original.
#[cfg(feature = "autoupdate")]
pub static MODULE_HANDLE: OnceLock<usize> = OnceLock::new();

#[cfg_attr(target_arch = "x86", crochet::hook("libavs-win32-ea3.dll", "XE592acd00008c"))]
#[cfg_attr(target_arch = "x86_64", crochet::hook("libavs-win64-ea3.dll", "XEyy2igh000007"))]
unsafe extern "C" fn avs_ea3_boot_startup_hook(node: *const ()) -> i32 {
    if let Err(err) = hook_init(node) {
        error!("{:#}", err);
    }

    call_original!(node)
}

#[no_mangle]
#[allow(non_snake_case, unused_variables)]
extern "system" fn DllMain(dll_module: HINSTANCE, call_reason: DWORD, reserved: LPVOID) -> BOOL {
    match call_reason {
        DLL_PROCESS_ATTACH => {
            unsafe { AllocConsole() };
            init_logger();
            print_infos();

            #[cfg(feature = "autoupdate")]
            let _ = MODULE_HANDLE.set(dll_module as usize);

            // If a previous run already downloaded+verified an update (see hook_init /
            // check_and_stage_update), apply it now, as the very first thing DllMain does —
            // before any network call, before AllocConsole/logging even matter to anyone but
            // us. This is a fresh process launch, so boot is nowhere near firing yet, and the
            // swap itself only ever kills a throwaway thread spawned right here — never the
            // boot thread — so there's nothing load-bearing on the other end of that kill.
            #[cfg(feature = "autoupdate")]
            let pending_apply = if CONFIGURATION.general.auto_update {
                let library_handle = unsafe { LibraryHandle::new(dll_module) };

                match updater::staged_update_path(&library_handle) {
                    Ok(path) if path.exists() => Some(library_handle),
                    Ok(_) => None,
                    Err(e) => {
                        error!("Could not check for a staged update: {e:#}");
                        None
                    }
                }
            } else {
                None
            };

            #[cfg(feature = "autoupdate")]
            match pending_apply {
                Some(library_handle) => {
                    info!("Applying previously downloaded update...");

                    // Don't enable the hook in this instance — we're about to unload it.
                    let thread_handle = ThreadHandle::duplicate_current_thread_handle();

                    thread::spawn(move || {
                        // Wait until DllMain returns and the loader lock is released before
                        // calling FreeLibrary on ourselves, since that (and the LoadLibraryW
                        // that follows once the old module is fully unloaded) can deadlock
                        // while the loader lock is held.
                        if let Ok(h) = thread_handle {
                            h.wait_and_close(50);
                        }

                        match updater::apply_staged_update(&library_handle) {
                            Ok(()) => {
                                info!("Update applied. Reloading into new hook...");
                                library_handle.free_and_exit_thread(1);
                            }
                            Err(e) => {
                                error!("Failed to apply staged update: {e:#}. Falling back to the current hook for this session.");

                                if let Err(err) = crochet::enable!(avs_ea3_boot_startup_hook) {
                                    error!("{:#}", err);
                                }
                            }
                        }
                    });
                }
                None => {
                    if let Err(err) = crochet::enable!(avs_ea3_boot_startup_hook) {
                        error!("{:#}", err);
                    }
                }
            }

            #[cfg(not(feature = "autoupdate"))]
            if let Err(err) = crochet::enable!(avs_ea3_boot_startup_hook) {
                error!("{:#}", err);
            }
        }
        DLL_PROCESS_DETACH => {
            if let Err(err) = crochet::disable!(avs_ea3_boot_startup_hook) {
                error!("{:#}", err);
            }

            if let Err(err) = hook_release() {
                error!("{:#}", err);
            }
        }
        _ => {}
    }

    TRUE
}
