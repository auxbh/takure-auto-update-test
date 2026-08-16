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

#[no_mangle]
#[allow(non_snake_case, unused_variables)]
extern "system" fn DllMain(dll_module: HINSTANCE, call_reason: DWORD, reserved: LPVOID) -> BOOL {
    match call_reason {
        DLL_PROCESS_ATTACH => {
            unsafe { AllocConsole() };
            init_logger();

            let library_handle = unsafe { LibraryHandle::new(dll_module) };
            let thread_handle = ThreadHandle::duplicate_current_thread_handle();

            thread::spawn(move || {
                // Wait until DllMain returns and the loader lock is released before doing
                // any real work, otherwise self-updating (which needs to spawn its own
                // thread and touch the filesystem) can deadlock.
                if let Ok(h) = thread_handle {
                    h.wait_and_close(1000);
                }

                print_infos();

                if let Err(err) = hook_init(library_handle) {
                    error!("{:#}", err);
                }
            });
        }
        DLL_PROCESS_DETACH => {
            if let Err(err) = hook_release() {
                error!("{:#}", err);
            }
        }
        _ => {}
    }

    TRUE
}
