use anyhow::Result;
use crate::{helpers, CONFIGURATION, TACHI_STATUS_URL};
use crate::handlers::scores::process_scores;
use crate::sys::{property_clear_error, property_mem_write, property_node_name, property_node_refer, property_query_size, property_search, property_set_flag, NodeType};
use crate::types::game::{Property2, Property3};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;
use log::{debug, error, info, warn};

pub static USER: AtomicU64 = AtomicU64::new(0);
pub static CURRENT_CARD_ID: RwLock<Option<String>> = RwLock::new(None);

pub fn hook_init(ea3_node: *const ()) -> Result<()> {
    if !CONFIGURATION.general.enable {
        return Ok(());
    }

    if let Some((model, dest, spec, revision, ext)) =
        helpers::read_node_str(ea3_node, b"/soft/model\0".as_ptr(), 3).and_then(|model| {
            let dest = helpers::read_node_str(ea3_node, b"/soft/dest\0".as_ptr(), 1)?;
            let spec = helpers::read_node_str(ea3_node, b"/soft/spec\0".as_ptr(), 1)?;
            let revision = helpers::read_node_str(ea3_node, b"/soft/rev\0".as_ptr(), 1)?;
            let ext = helpers::read_node_str(ea3_node, b"/soft/ext\0".as_ptr(), 10)?
                .parse::<u64>()
                .unwrap_or(0);
            Some((model, dest, spec, revision, ext))
        })
    {
        if model != "MDX" || revision == "O" || revision == "X" || ext < 2022022801 {
            error!(
                "Unsupported model/revision/ext '{}:{}:{}:{}:{}', hook will not be enabled",
                model, dest, spec, revision, ext
            );
            return Ok(());
        } else {
            info!(
                "Detected game software '{}:{}:{}:{}:{}'",
                model, dest, spec, revision, ext
            );
        }
    } else {
        warn!("Could not read game version, hook might not work properly");
    }

    // Trying to reach Tachi API
    if cfg!(debug_assertions) {
        info!("Debug mode is enabled, not reaching Tachi API");
    } else {
        let response: serde_json::Value =
            helpers::request_tachi("GET", TACHI_STATUS_URL.as_str(), None::<()>)?;
        let user = response["body"]["whoami"]
            .as_u64()
            .ok_or(anyhow::anyhow!("Couldn't parse user from Tachi response"))?;
        USER.store(user, Ordering::Relaxed);
        info!("Tachi API successfully reached, user {}", user);
    }

    // Initializing function detours
    crochet::enable!(property_destroy_hook)
        .map_err(|err| anyhow::anyhow!("Could not enable function detour: {:#}", err))?;

    info!("Hook successfully initialized");

    Ok(())
}

pub fn hook_release() -> Result<()> {
    if !CONFIGURATION.general.enable {
        return Ok(());
    }

    if crochet::is_enabled!(property_destroy_hook) {
        crochet::disable!(property_destroy_hook)
            .map_err(|err| anyhow::anyhow!("Could not disable function detour: {:#}", err))?;
    }

    Ok(())
}

#[cfg_attr(target_arch = "x86", crochet::hook("libavs-win32.dll", "XCd229cc00013c"))]
#[cfg_attr(target_arch = "x86_64", crochet::hook("libavs-win64.dll", "XCnbrep7000091"))]
pub unsafe extern "C" fn property_destroy_hook(property: *mut ()) -> i32 {
    if property.is_null() {
        return 0;
    }

    let node = {
        let p2 = property_search(property, std::ptr::null(), b"/call/playerdata_2\0".as_ptr());
        if !p2.is_null() {
            p2
        } else {
            let p3 = property_search(property, std::ptr::null(), b"/call/playdata_3\0".as_ptr());
            if !p3.is_null() {
                p3
            } else {
                property_search(property, std::ptr::null(), b"/call/cardmng\0".as_ptr())
            }
        }
    };
    
    if node.is_null() {
        property_clear_error(property);
        return call_original!(property);
    }

    let mut buffer = [0u8; 256];
    let result = property_node_name(node, buffer.as_mut_ptr(), buffer.len() as u32);
    if result < 0 {
        return call_original!(property);
    }

    let name = {
        let result = std::str::from_utf8(&buffer[0..32]);
        if let Err(err) = result {
            error!("Could not convert buffer to string: {:#}", err);
            return call_original!(property);
        }
        result.unwrap().replace('\0', "")
    };

    if name != "playerdata_2" && name != "playdata_3" && name != "cardmng" {
        return call_original!(property);
    }

    let result = property_node_refer(
        property,
        node,
        b"method@\0".as_ptr(),
        NodeType::NodeAttr,
        buffer.as_mut_ptr() as *mut (),
        256,
    );
    if result < 0 {
        return call_original!(property);
    }

    let method = {
        let result = std::str::from_utf8(&buffer[0..21]);
        if let Err(err) = result {
            error!("Could not convert buffer to string: {:#}", err);
            return call_original!(property);
        }
        result.unwrap().replace('\0', "")
    };

    debug!("Intercepted '{}' method: {}", name, method);

    if name == "cardmng" {
        if method != "inquire" {
            return call_original!(property);
        }

        let result = property_node_refer(
            property,
            node,
            b"cardid@\0".as_ptr(),
            NodeType::NodeAttr,
            buffer.as_mut_ptr() as *mut (),
            256,
        );
        if result < 0 {
            return call_original!(property);
        }

        let cardid = {
            let result = std::str::from_utf8(&buffer[..32]);
            if let Err(err) = result {
                error!("Could not convert buffer to string: {:#}", err);
                return call_original!(property);
            }

            result.unwrap().replace('\0', "")
        };

        if let Ok(mut guard) = CURRENT_CARD_ID.write() {
            debug!("Set current card id to {}", cardid);
            *guard = Some(cardid);
        } else {
            warn!("Could not acquire write lock on current card id");
        }

        return call_original!(property);
    }

    if method != "usergamedata_advanced" && method != "playerdata_save" {
        return call_original!(property);
    }

    property_set_flag(property, 0x800, 0x008);

    let size = property_query_size(property);
    if size < 0 {
        property_set_flag(property, 0x008, 0x800);
        return call_original!(property);
    }

    let buffer = vec![0u8; size as usize];
    let result = property_mem_write(property, buffer.as_ptr() as *mut u8, buffer.len() as u32);

    property_set_flag(property, 0x008, 0x800);

    if result < 0 {
        return call_original!(property);
    }

    // Read buffer to string
    let property_str = {
        let result = std::str::from_utf8(&buffer);
        if let Err(err) = result {
            error!("Could not convert buffer to string: {:#}", err);
            return call_original!(property);
        }
        result.unwrap()
    };

    debug!("Processing property: {}", property_str);
    if let Err(err) = match method.as_str() {
        "usergamedata_advanced" => serde_json::from_str::<Property2>(property_str)
        .map_err(|err| anyhow::anyhow!("Could not parse property: {:#}", err))
        .and_then(|prop| {
            debug!("Mode: {:#?}", prop.call.playerdata_2.data.mode);
            if prop.call.playerdata_2.data.mode != "usersave" {
                return Ok(());
            }
            debug!("Filtered Property: {:#?}", prop);
            process_scores(either::Left(prop.call.playerdata_2.data))
        }),
        "playerdata_save" => serde_json::from_str::<Property3>(property_str)
        .map_err(|err| anyhow::anyhow!("Could not parse property: {:#}", err))
        .and_then(|prop| {
            debug!("Savekind: {:#?}", prop.call.playdata_3.data.savekind);
            if prop.call.playdata_3.data.savekind != 2 {
                return Ok(());
            }
            debug!("Filtered Property: {:#?}", prop);
            process_scores(either::Right(prop.call.playdata_3.data))
        }),
    _ => unreachable!(),
    } {
        error!("{:#}", err);
    }
    call_original!(property)
}