use std::net::UdpSocket;
use std::time::{Instant, Duration};
use anyhow::{bail, Result};
use serde_json::json;
use serde_json::Value;

use rosc::{OscPacket, OscType, OscMessage};
use crate::api::client::{qs, ApiClient};
use crate::cli::{GlobalArgs, OscCmd};

struct BucketedState {
    run_ping: bool,
    run_whoami: bool,
    latest_layout_switch: Option<OscMessage>,
    deletes: std::collections::HashSet<u32>,
    element_updates: std::collections::HashMap<u32, serde_json::Map<String, Value>>,
}

impl BucketedState {
    fn new() -> Self {
        Self {
            run_ping: false,
            run_whoami: false,
            latest_layout_switch: None,
            deletes: std::collections::HashSet::new(),
            element_updates: std::collections::HashMap::new(),
        }
    }
}

pub fn run(cmd: OscCmd, global: &GlobalArgs) -> Result<()> {
    let host = &cmd.host;
    let port = cmd.port;
    let addr = format!("{host}:{port}");

    let socket = UdpSocket::bind(&addr)?;
    println!("OSC listener running on {addr}");
    println!("Throttling API updates to 30Hz (matching the native server-side update rate).");
    println!("Press Ctrl+C to stop.");

    let client = super::client_for(global)?;

    let mut last_execution = Instant::now() - Duration::from_millis(34);
    let min_interval = Duration::from_millis(33); // 30Hz

    let mut last_sent_elements: std::collections::HashMap<u32, serde_json::Map<String, Value>> = std::collections::HashMap::new();

    let mut buf = [0u8; 65535];
    loop {
        // Block waiting for the first packet
        let (size, src) = match socket.recv_from(&mut buf) {
            Ok(res) => res,
            Err(e) => {
                eprintln!("Socket receive error: {e}");
                continue;
            }
        };

        // Decode the first packet
        let mut pending_packets = Vec::new();
        match rosc::decoder::decode_udp(&buf[..size]) {
            Ok((_, packet)) => pending_packets.push(packet),
            Err(e) => eprintln!("Error decoding OSC packet from {src}: {e:#}"),
        }

        // Set to non-blocking to drain the buffer of any queued packets
        if let Err(e) = socket.set_nonblocking(true) {
            eprintln!("Failed to set non-blocking: {e}");
        }

        // Drain all pending packets from the OS buffer
        loop {
            match socket.recv_from(&mut buf) {
                Ok((s_size, s_src)) => {
                    match rosc::decoder::decode_udp(&buf[..s_size]) {
                        Ok((_, packet)) => pending_packets.push(packet),
                        Err(e) => eprintln!("Error decoding OSC packet from {s_src}: {e:#}"),
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    break;
                }
                Err(e) => {
                    eprintln!("Error draining socket: {e}");
                    break;
                }
            }
        }

        // Reset back to blocking
        if let Err(e) = socket.set_nonblocking(false) {
            eprintln!("Failed to set blocking: {e}");
        }

        // Process and bucket all packets into a single merged state
        let mut bucketed_state = BucketedState::new();
        for packet in pending_packets {
            process_packet_into_bucket(packet, &mut bucketed_state);
        }

        // Enforce the 30Hz rate limit before executing
        let elapsed = last_execution.elapsed();
        if elapsed < min_interval {
            std::thread::sleep(min_interval - elapsed);
        }
        last_execution = Instant::now();

        // Execute layout switch first
        if let Some(msg) = bucketed_state.latest_layout_switch {
            if let Err(e) = handle_layout_switch(msg, &client) {
                eprintln!("Error switching layout: {e:#}");
            }
        }

        // Execute pings and whoami
        if bucketed_state.run_ping {
            if let Err(e) = run_ping_action(&client) {
                eprintln!("Error executing ping: {e:#}");
            }
        }
        if bucketed_state.run_whoami {
            if let Err(e) = run_whoami_action(&client) {
                eprintln!("Error executing whoami: {e:#}");
            }
        }

        // Execute deletes
        for id in bucketed_state.deletes {
            if let Err(e) = client.delete(&format!("elements?id={id}")) {
                eprintln!("Error deleting element {id}: {e:#}");
            } else {
                println!("Deleted element {id}");
                last_sent_elements.remove(&id);
            }
        }

        // Execute element updates (merged states)
        for (id, fields_map) in bucketed_state.element_updates {
            if fields_map.is_empty() {
                continue;
            }

            // Get cached state for this element
            let cached = last_sent_elements.entry(id).or_insert_with(serde_json::Map::new);

            // Filter out fields that do not exceed the deadband threshold
            let mut filtered_map = serde_json::Map::new();
            for (key, val) in fields_map {
                let cached_val = cached.get(&key).unwrap_or(&Value::Null);
                if should_update_field(&key, &val, cached_val) {
                    filtered_map.insert(key.clone(), val.clone());
                }
            }

            if filtered_map.is_empty() {
                continue;
            }

            // Update the cache with the new values
            let mut cached_val = Value::Object(cached.clone());
            json_merge(&mut cached_val, Value::Object(filtered_map.clone()));
            if let Value::Object(merged_cached) = cached_val {
                *cached = merged_cached;
            }

            let body = Value::Object(filtered_map);
            if let Err(e) = client.patch(&format!("elements?id={id}"), body) {
                eprintln!("Error updating element {id}: {e:#}");
            } else {
                println!("Updated element {id} (merged state)");
            }
        }
    }
}

fn process_packet_into_bucket(packet: OscPacket, state: &mut BucketedState) {
    match packet {
        OscPacket::Message(msg) => {
            if let Err(e) = merge_message_into_bucket(msg, state) {
                eprintln!("Error bucketing message: {e:#}");
            }
        }
        OscPacket::Bundle(bundle) => {
            for inner in bundle.content {
                process_packet_into_bucket(inner, state);
            }
        }
    }
}

fn merge_message_into_bucket(msg: OscMessage, state: &mut BucketedState) -> Result<()> {
    let addr = msg.addr.as_str();

    match addr {
        "/pogly/ping" | "/ping" => {
            state.run_ping = true;
        }
        "/pogly/whoami" | "/whoami" => {
            state.run_whoami = true;
        }
        "/pogly/layouts/set-active" | "/pogly/layouts/set_active" | "/pogly/active_layout" => {
            state.latest_layout_switch = Some(msg);
        }
        "/pogly/elements/delete" => {
            if msg.args.is_empty() {
                bail!("Expected 1 argument (element ID) for delete");
            }
            let id = coerce_to_u32(&msg.args[0])?;
            state.deletes.insert(id);
            state.element_updates.remove(&id);
        }
        "/pogly/elements/update/position" => {
            if msg.args.len() < 3 {
                bail!("Expected 3 arguments (id, x, y) for position update");
            }
            let id = coerce_to_u32(&msg.args[0])?;
            if state.deletes.contains(&id) {
                return Ok(());
            }
            let x = coerce_to_i64(&msg.args[1])?;
            let y = coerce_to_i64(&msg.args[2])?;

            let val = json!({
                "x": x,
                "y": y
            });
            merge_element_update(state, id, val);
        }
        "/pogly/elements/update/transparency" => {
            if msg.args.len() < 2 {
                bail!("Expected 2 arguments (id, transparency) for transparency update");
            }
            let id = coerce_to_u32(&msg.args[0])?;
            if state.deletes.contains(&id) {
                return Ok(());
            }
            let transparency = coerce_to_i64(&msg.args[1])?;

            let val = json!({
                "transparency": transparency
            });
            merge_element_update(state, id, val);
        }
        "/pogly/elements/update/text" => {
            if msg.args.len() < 2 {
                bail!("Expected 2 arguments (id, text) for text update");
            }
            let id = coerce_to_u32(&msg.args[0])?;
            if state.deletes.contains(&id) {
                return Ok(());
            }
            let text = coerce_to_string(&msg.args[1])?;

            let val = json!({
                "text": {
                    "text": text
                }
            });
            merge_element_update(state, id, val);
        }
        "/pogly/elements/update/media/playing" => {
            if msg.args.len() < 2 {
                bail!("Expected 2 arguments (id, playing) for media playing update");
            }
            let id = coerce_to_u32(&msg.args[0])?;
            if state.deletes.contains(&id) {
                return Ok(());
            }
            let playing = coerce_to_bool(&msg.args[1])?;

            let val = json!({
                "media": {
                    "playing": playing
                }
            });
            merge_element_update(state, id, val);
        }
        "/pogly/elements/update/media/volume" => {
            if msg.args.len() < 2 {
                bail!("Expected 2 arguments (id, volume) for media volume update");
            }
            let id = coerce_to_u32(&msg.args[0])?;
            if state.deletes.contains(&id) {
                return Ok(());
            }
            let volume = coerce_to_i64(&msg.args[1])?;

            let val = json!({
                "media": {
                    "volume": volume
                }
            });
            merge_element_update(state, id, val);
        }
        "/pogly/elements/update/media/timestamp" => {
            if msg.args.len() < 2 {
                bail!("Expected 2 arguments (id, timestamp) for media timestamp update");
            }
            let id = coerce_to_u32(&msg.args[0])?;
            if state.deletes.contains(&id) {
                return Ok(());
            }
            let timestamp = coerce_to_i64(&msg.args[1])?;

            let val = json!({
                "media": {
                    "timestamp": timestamp
                }
            });
            merge_element_update(state, id, val);
        }
        "/pogly/elements/update" => {
            if msg.args.len() < 3 {
                bail!("Expected 3 arguments (id, field_name, value) for generic update");
            }
            let id = coerce_to_u32(&msg.args[0])?;
            if state.deletes.contains(&id) {
                return Ok(());
            }
            let field_name = coerce_to_string(&msg.args[1])?;
            let raw_value = &msg.args[2];

            let val = build_generic_update_body(&field_name, raw_value)?;
            merge_element_update(state, id, val);
        }
        _ => {
            println!("Unknown OSC address: {addr}");
        }
    }

    Ok(())
}

fn merge_element_update(state: &mut BucketedState, id: u32, val: Value) {
    let entry = state.element_updates.entry(id).or_insert_with(serde_json::Map::new);
    let mut entry_val = Value::Object(entry.clone());
    json_merge(&mut entry_val, val);
    if let Value::Object(merged_map) = entry_val {
        *entry = merged_map;
    }
}

fn json_merge(target: &mut Value, source: Value) {
    match (target, source) {
        (Value::Object(target_map), Value::Object(source_map)) => {
            for (key, val) in source_map {
                if !target_map.contains_key(&key) {
                    target_map.insert(key, val);
                } else {
                    json_merge(target_map.get_mut(&key).unwrap(), val);
                }
            }
        }
        (target, source) => {
            *target = source;
        }
    }
}

fn handle_layout_switch(msg: OscMessage, client: &ApiClient) -> Result<()> {
    if msg.args.is_empty() {
        bail!("Expected 1 argument (layout name or ID)");
    }
    let arg = &msg.args[0];
    let response = if let Ok(id) = coerce_to_u32(arg) {
        let query = qs(&[("id", Some(id.to_string())), ("name", None)]);
        client.post(&format!("layout{query}"), json!({}))?
    } else {
        let name = coerce_to_string(arg)?;
        let query = qs(&[("id", None), ("name", Some(name))]);
        client.post(&format!("layout{query}"), json!({}))?
    };
    let layout = &response["activeLayout"];
    println!(
        "Switched to layout: {} '{}'",
        layout["id"],
        layout["name"].as_str().unwrap_or("")
    );
    Ok(())
}

fn run_ping_action(client: &ApiClient) -> Result<()> {
    let response = client.get("ping")?;
    println!("Ping response: {response}");
    Ok(())
}

fn run_whoami_action(client: &ApiClient) -> Result<()> {
    let response = client.get("whoami")?;
    println!("Whoami response: {response}");
    Ok(())
}

fn should_update_field(key: &str, new_val: &Value, cached_val: &Value) -> bool {
    if new_val == cached_val {
        return false;
    }

    match key {
        "x" | "y" | "transparency" => {
            if let (Some(n), Some(c)) = (new_val.as_i64(), cached_val.as_i64()) {
                return (n - c).abs() > 1; // Deadband: must change by at least 2 units
            }
        }
        "media" => {
            if let (Some(n_obj), Some(c_obj)) = (new_val.as_object(), cached_val.as_object()) {
                for (sub_key, sub_val) in n_obj {
                    if let Some(c_val) = c_obj.get(sub_key) {
                        if sub_key == "volume" || sub_key == "timestamp" {
                            if let (Some(n), Some(c)) = (sub_val.as_i64(), c_val.as_i64()) {
                                if (n - c).abs() > 1 {
                                    return true;
                                }
                            } else if sub_val != c_val {
                                return true;
                            }
                        } else if sub_val != c_val {
                            return true;
                        }
                    } else {
                        return true;
                    }
                }
                return false;
            }
        }
        "text" => {
            if let (Some(n_obj), Some(c_obj)) = (new_val.as_object(), cached_val.as_object()) {
                if let (Some(n_val), Some(c_val)) = (n_obj.get("size"), c_obj.get("size")) {
                    if let (Some(n), Some(c)) = (n_val.as_i64(), c_val.as_i64()) {
                        if (n - c).abs() > 1 {
                            return true;
                        }
                    }
                }
            }
        }
        _ => {}
    }

    new_val != cached_val
}

fn coerce_to_string(arg: &OscType) -> Result<String> {
    match arg {
        OscType::String(s) => Ok(s.clone()),
        OscType::Int(i) => Ok(i.to_string()),
        OscType::Long(l) => Ok(l.to_string()),
        OscType::Float(f) => Ok(f.to_string()),
        OscType::Double(d) => Ok(d.to_string()),
        OscType::Bool(b) => Ok(b.to_string()),
        _ => bail!("Cannot coerce OSC arg {:?} to string", arg),
    }
}

fn coerce_to_u32(arg: &OscType) -> Result<u32> {
    match arg {
        OscType::Int(i) => Ok(*i as u32),
        OscType::Long(l) => Ok(*l as u32),
        OscType::Float(f) => Ok(*f as u32),
        OscType::Double(d) => Ok(*d as u32),
        OscType::String(s) => s.parse::<u32>().map_err(|e| anyhow::anyhow!("Failed to parse string to u32: {e}")),
        _ => bail!("Cannot coerce OSC arg {:?} to u32", arg),
    }
}

fn coerce_to_i64(arg: &OscType) -> Result<i64> {
    match arg {
        OscType::Int(i) => Ok(*i as i64),
        OscType::Long(l) => Ok(*l),
        OscType::Float(f) => Ok(*f as i64),
        OscType::Double(d) => Ok(*d as i64),
        OscType::String(s) => s.parse::<i64>().map_err(|e| anyhow::anyhow!("Failed to parse string to i64: {e}")),
        _ => bail!("Cannot coerce OSC arg {:?} to i64", arg),
    }
}

fn coerce_to_bool(arg: &OscType) -> Result<bool> {
    match arg {
        OscType::Bool(b) => Ok(*b),
        OscType::Int(i) => Ok(*i != 0),
        OscType::Long(l) => Ok(*l != 0),
        OscType::Float(f) => Ok(*f != 0.0),
        OscType::Double(d) => Ok(*d != 0.0),
        OscType::String(s) => {
            let s_lower = s.to_lowercase();
            if s_lower == "true" || s_lower == "1" {
                Ok(true)
            } else if s_lower == "false" || s_lower == "0" {
                Ok(false)
            } else {
                bail!("Invalid boolean string: {s}")
            }
        }
        _ => bail!("Cannot coerce OSC arg {:?} to bool", arg),
    }
}

fn build_generic_update_body(field_name: &str, raw_value: &OscType) -> Result<Value> {
    let mut body = serde_json::Map::new();

    match field_name {
        // Root fields
        "x" | "y" | "transparency" | "layout_id" | "layoutId" => {
            let val = coerce_to_i64(raw_value)?;
            body.insert(
                if field_name == "layout_id" { "layoutId".to_string() } else { field_name.to_string() },
                json!(val)
            );
        }
        "transform" | "clip" => {
            let val = coerce_to_string(raw_value)?;
            body.insert(field_name.to_string(), json!(val));
        }
        "locked" | "always_loaded" | "alwaysLoaded" | "index_lock" | "indexLock" => {
            let val = coerce_to_bool(raw_value)?;
            let key = match field_name {
                "always_loaded" => "alwaysLoaded",
                "index_lock" => "indexLock",
                other => other,
            };
            body.insert(key.to_string(), json!(val));
        }

        // Text group fields
        "text" => {
            let val = coerce_to_string(raw_value)?;
            body.insert("text".to_string(), json!({ "text": val }));
        }
        "text_size" | "textSize" | "size" => {
            let val = coerce_to_i64(raw_value)?;
            body.insert("text".to_string(), json!({ "size": val }));
        }
        "color" | "font" | "css" => {
            let val = coerce_to_string(raw_value)?;
            body.insert("text".to_string(), json!({ field_name: val }));
        }

        // Image group fields
        "image_data_id" | "imageDataId" | "image_element_data_id" => {
            let val = coerce_to_u32(raw_value)?;
            body.insert("image".to_string(), json!({ "elementDataId": val }));
        }
        "image_url" | "imageUrl" | "url" => {
            let val = coerce_to_string(raw_value)?;
            body.insert("image".to_string(), json!({ "url": val }));
        }
        "image_width" | "imageWidth" => {
            let val = coerce_to_i64(raw_value)?;
            body.insert("image".to_string(), json!({ "width": val }));
        }
        "image_height" | "imageHeight" => {
            let val = coerce_to_i64(raw_value)?;
            body.insert("image".to_string(), json!({ "height": val }));
        }

        // Widget group fields
        "widget_data_id" | "widgetDataId" | "widget_element_data_id" => {
            let val = coerce_to_u32(raw_value)?;
            body.insert("widget".to_string(), json!({ "elementDataId": val }));
        }
        "widget_raw_data" | "widgetRawData" | "raw_data" => {
            let val = coerce_to_string(raw_value)?;
            body.insert("widget".to_string(), json!({ "rawData": val }));
        }
        "widget_width" | "widgetWidth" => {
            let val = coerce_to_i64(raw_value)?;
            body.insert("widget".to_string(), json!({ "width": val }));
        }
        "widget_height" | "widgetHeight" => {
            let val = coerce_to_i64(raw_value)?;
            body.insert("widget".to_string(), json!({ "height": val }));
        }

        // Media group fields
        "media_source" | "mediaSource" | "source" => {
            let val = coerce_to_string(raw_value)?;
            body.insert("media".to_string(), json!({ "source": val }));
        }
        "media_volume" | "mediaVolume" | "volume" => {
            let val = coerce_to_i64(raw_value)?;
            body.insert("media".to_string(), json!({ "volume": val }));
        }
        "media_playing" | "mediaPlaying" | "playing" => {
            let val = coerce_to_bool(raw_value)?;
            body.insert("media".to_string(), json!({ "playing": val }));
        }
        "media_timestamp" | "mediaTimestamp" | "timestamp" => {
            let val = coerce_to_i64(raw_value)?;
            body.insert("media".to_string(), json!({ "timestamp": val }));
        }
        "media_autoplay" | "mediaAutoplay" | "autoplay" => {
            let val = coerce_to_bool(raw_value)?;
            body.insert("media".to_string(), json!({ "autoplay": val }));
        }
        "media_loop" | "mediaLoop" | "loop" => {
            let val = coerce_to_bool(raw_value)?;
            body.insert("media".to_string(), json!({ "loop": val }));
        }
        "media_width" | "mediaWidth" => {
            let val = coerce_to_i64(raw_value)?;
            body.insert("media".to_string(), json!({ "width": val }));
        }
        "media_height" | "mediaHeight" => {
            let val = coerce_to_i64(raw_value)?;
            body.insert("media".to_string(), json!({ "height": val }));
        }

        _ => bail!("Unknown element update field: {field_name}"),
    }

    Ok(Value::Object(body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coerce_to_string() {
        assert_eq!(coerce_to_string(&OscType::String("test".into())).unwrap(), "test");
        assert_eq!(coerce_to_string(&OscType::Int(123)).unwrap(), "123");
        assert_eq!(coerce_to_string(&OscType::Long(456)).unwrap(), "456");
        assert_eq!(coerce_to_string(&OscType::Float(1.23)).unwrap(), "1.23");
        assert_eq!(coerce_to_string(&OscType::Double(4.56)).unwrap(), "4.56");
        assert_eq!(coerce_to_string(&OscType::Bool(true)).unwrap(), "true");
    }

    #[test]
    fn test_coerce_to_u32() {
        assert_eq!(coerce_to_u32(&OscType::Int(123)).unwrap(), 123);
        assert_eq!(coerce_to_u32(&OscType::Long(456)).unwrap(), 456);
        assert_eq!(coerce_to_u32(&OscType::Float(7.89)).unwrap(), 7);
        assert_eq!(coerce_to_u32(&OscType::Double(9.87)).unwrap(), 9);
        assert_eq!(coerce_to_u32(&OscType::String("654".into())).unwrap(), 654);
    }

    #[test]
    fn test_coerce_to_i64() {
        assert_eq!(coerce_to_i64(&OscType::Int(123)).unwrap(), 123);
        assert_eq!(coerce_to_i64(&OscType::Long(456)).unwrap(), 456);
        assert_eq!(coerce_to_i64(&OscType::Float(7.89)).unwrap(), 7);
        assert_eq!(coerce_to_i64(&OscType::Double(9.87)).unwrap(), 9);
        assert_eq!(coerce_to_i64(&OscType::String("654".into())).unwrap(), 654);
    }

    #[test]
    fn test_coerce_to_bool() {
        assert_eq!(coerce_to_bool(&OscType::Bool(true)).unwrap(), true);
        assert_eq!(coerce_to_bool(&OscType::Int(1)).unwrap(), true);
        assert_eq!(coerce_to_bool(&OscType::Int(0)).unwrap(), false);
        assert_eq!(coerce_to_bool(&OscType::Float(0.0)).unwrap(), false);
        assert_eq!(coerce_to_bool(&OscType::Float(1.0)).unwrap(), true);
        assert_eq!(coerce_to_bool(&OscType::String("true".into())).unwrap(), true);
        assert_eq!(coerce_to_bool(&OscType::String("0".into())).unwrap(), false);
    }

    #[test]
    fn test_build_generic_update_body() {
        let body = build_generic_update_body("x", &OscType::Int(120)).unwrap();
        assert_eq!(body, json!({ "x": 120 }));

        let body = build_generic_update_body("text", &OscType::String("hello".into())).unwrap();
        assert_eq!(body, json!({ "text": { "text": "hello" } }));

        let body = build_generic_update_body("always_loaded", &OscType::Bool(true)).unwrap();
        assert_eq!(body, json!({ "alwaysLoaded": true }));

        let body = build_generic_update_body("media_playing", &OscType::Int(1)).unwrap();
        assert_eq!(body, json!({ "media": { "playing": true } }));
    }

    #[test]
    fn test_json_merge() {
        let mut target = json!({
            "x": 10,
            "media": {
                "playing": true
            }
        });
        
        let source = json!({
            "y": 20,
            "media": {
                "volume": 80
            }
        });

        json_merge(&mut target, source);

        assert_eq!(target, json!({
            "x": 10,
            "y": 20,
            "media": {
                "playing": true,
                "volume": 80
            }
        }));
    }

    #[test]
    fn test_should_update_field() {
        // Test identical values (should not update)
        assert!(!should_update_field("x", &json!(10), &json!(10)));

        // Test minor changes within deadband (should not update)
        assert!(!should_update_field("x", &json!(11), &json!(10)));
        assert!(!should_update_field("x", &json!(9), &json!(10)));

        // Test larger changes exceeding deadband (should update)
        assert!(should_update_field("x", &json!(12), &json!(10)));
        assert!(should_update_field("x", &json!(8), &json!(10)));

        // Test non-numeric changes (should update)
        assert!(should_update_field("transform", &json!("scale(1)"), &json!("scale(2)")));

        // Test nested objects like media volume within deadband
        let c_media = json!({"volume": 50});
        let n_media_small = json!({"volume": 51});
        let n_media_large = json!({"volume": 55});
        assert!(!should_update_field("media", &n_media_small, &c_media));
        assert!(should_update_field("media", &n_media_large, &c_media));
    }
}
