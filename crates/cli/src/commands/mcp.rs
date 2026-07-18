use anyhow::{bail, Result};
use serde_json::{json, Map, Value};
use std::io::{self, BufRead, Write};

use crate::api::client::{qs, ApiClient};
use crate::cli::GlobalArgs;
use crate::config::Config;

pub fn run(_global: &GlobalArgs) -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let handle = stdin.lock();

    for line in handle.lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Error reading line from stdin: {}", e);
                break;
            }
        };

        let request: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                let response = json!({
                    "jsonrpc": "2.0",
                    "error": {
                        "code": -32700,
                        "message": format!("Parse error: {}", e)
                    }
                });
                if let Err(write_err) = writeln!(stdout, "{}", response) {
                    eprintln!("Error writing response to stdout: {}", write_err);
                    break;
                }
                let _ = io::Write::flush(&mut stdout);
                continue;
            }
        };

        let has_id = request.get("id").is_some();
        let response = handle_mcp_request(request);

        if has_id && response != Value::Null {
            if let Err(e) = writeln!(stdout, "{}", response) {
                eprintln!("Error writing response to stdout: {}", e);
                break;
            }
            let _ = io::Write::flush(&mut stdout);
        }
    }
    Ok(())
}

fn handle_mcp_request(req: Value) -> Value {
    let id = req.get("id").cloned().unwrap_or(Value::Null);
    let method = match req.get("method").and_then(Value::as_str) {
        Some(m) => m,
        None => {
            return json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32600,
                    "message": "Invalid request: missing method"
                }
            });
        }
    };

    let params = req.get("params").cloned().unwrap_or(Value::Null);

    match method {
        "initialize" => {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": "2025-06-18",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": "pogly-mcp",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                }
            })
        }
        "notifications/initialized" => Value::Null,
        "tools/list" => {
            let tools = get_tools_list();
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "tools": tools
                }
            })
        }
        "tools/call" => {
            let tool_name = match params.get("name").and_then(Value::as_str) {
                Some(n) => n,
                None => {
                    return json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {
                            "code": -32602,
                            "message": "Invalid params: missing name"
                        }
                    });
                }
            };
            let args = params.get("arguments").cloned().unwrap_or(Value::Null);

            match execute_tool(tool_name, args) {
                Ok(res) => {
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "content": [
                                {
                                    "type": "text",
                                    "text": res.to_string()
                                }
                            ]
                        }
                    })
                }
                Err(e) => {
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "content": [
                                {
                                    "type": "text",
                                    "text": format!("Error: {:#}", e)
                                }
                            ],
                            "isError": true
                        }
                    })
                }
            }
        }
        _ => {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32601,
                    "message": format!("Method not found: {}", method)
                }
            })
        }
    }
}

fn get_client(overlay: Option<String>) -> Result<ApiClient> {
    let config = Config::load()?;
    let profile = config.resolve(overlay.as_deref())?;
    Ok(ApiClient::new(&profile.address, &profile.token))
}

fn execute_tool(name: &str, args: Value) -> Result<Value> {
    let overlay = args
        .get("overlay")
        .and_then(Value::as_str)
        .map(String::from);
    let client = get_client(overlay)?;

    match name {
        "ping" => client.get("ping"),
        "whoami" => client.get("whoami"),
        "layouts_list" => client.get("layouts"),
        "layouts_add" => {
            let layout_name = args
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow::anyhow!("missing name"))?;
            let active = args.get("active").and_then(Value::as_bool).unwrap_or(false);
            let mut body = json!({ "name": layout_name });
            if active {
                body["active"] = json!(true);
            }
            client.post("layouts", body)
        }
        "layouts_duplicate" => {
            let id = args
                .get("id")
                .and_then(Value::as_u64)
                .ok_or_else(|| anyhow::anyhow!("missing id"))?;
            client.post(&format!("layouts/duplicate?id={id}"), json!({}))
        }
        "layouts_rename" => {
            let id = args
                .get("id")
                .and_then(Value::as_u64)
                .ok_or_else(|| anyhow::anyhow!("missing id"))?;
            let new_name = args
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow::anyhow!("missing name"))?;
            client.patch(&format!("layouts?id={id}"), json!({ "name": new_name }))
        }
        "layouts_delete" => {
            let id = args
                .get("id")
                .and_then(Value::as_u64)
                .ok_or_else(|| anyhow::anyhow!("missing id"))?;
            let preserve_elements = args
                .get("preserve_elements")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let preserve_layout_id = args.get("preserve_layout_id").and_then(Value::as_u64);
            let query = qs(&[
                ("id", Some(id.to_string())),
                (
                    "preserveElements",
                    preserve_elements.then(|| "true".to_string()),
                ),
                (
                    "preserveLayoutId",
                    preserve_layout_id.map(|v| v.to_string()),
                ),
            ]);
            client.delete(&format!("layouts{query}"))
        }
        "layouts_set_active" => {
            let id = args.get("id").and_then(Value::as_u64);
            let layout_name = args.get("name").and_then(Value::as_str);
            if id.is_none() && layout_name.is_none() {
                bail!("provide either id or name");
            }
            let query = qs(&[
                ("id", id.map(|v| v.to_string())),
                ("name", layout_name.map(String::from)),
            ]);
            client.post(&format!("layout{query}"), json!({}))
        }
        "elements_list" => {
            let id = args.get("id").and_then(Value::as_u64);
            let layout = args.get("layout").and_then(Value::as_u64);
            let query = qs(&[
                ("id", id.map(|v| v.to_string())),
                ("layout", layout.map(|v| v.to_string())),
            ]);
            client.get(&format!("elements{query}"))
        }
        "elements_delete" => {
            let id = args
                .get("id")
                .and_then(Value::as_u64)
                .ok_or_else(|| anyhow::anyhow!("missing id"))?;
            client.delete(&format!("elements?id={id}"))
        }
        "elements_add_text" => {
            let text = args
                .get("text")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow::anyhow!("missing text"))?;
            let size = args.get("size").and_then(Value::as_i64);
            let color = args.get("color").and_then(Value::as_str).map(String::from);
            let font = args.get("font").and_then(Value::as_str).map(String::from);
            let css = args.get("css").and_then(Value::as_str).map(String::from);

            let mut text_group = Map::new();
            text_group.insert("text".into(), text.into());
            if let Some(s) = size {
                text_group.insert("size".into(), s.into());
            }
            if let Some(c) = color {
                text_group.insert("color".into(), c.into());
            }
            if let Some(f) = font {
                text_group.insert("font".into(), f.into());
            }
            if let Some(cs) = css {
                text_group.insert("css".into(), cs.into());
            }

            let mut body = Map::new();
            body.insert("type".into(), "text".into());
            body.insert("text".into(), Value::Object(text_group));
            apply_common_mcp(&mut body, &args);

            client.post("elements", Value::Object(body))
        }
        "elements_add_image" => {
            let data_id = args.get("data_id").and_then(Value::as_u64);
            let url = args.get("url").and_then(Value::as_str).map(String::from);
            let width = args.get("width").and_then(Value::as_i64);
            let height = args.get("height").and_then(Value::as_i64);

            if url.is_some() && (width.is_none() || height.is_none()) {
                bail!("url requires width and height");
            }
            if data_id.is_none() && url.is_none() {
                bail!("provide either data_id or url");
            }

            let mut image_group = Map::new();
            if let Some(d) = data_id {
                image_group.insert("elementDataId".into(), d.into());
            }
            if let Some(u) = url {
                image_group.insert("url".into(), u.into());
            }
            if let Some(w) = width {
                image_group.insert("width".into(), w.into());
            }
            if let Some(h) = height {
                image_group.insert("height".into(), h.into());
            }

            let mut body = Map::new();
            body.insert("type".into(), "image".into());
            body.insert("image".into(), Value::Object(image_group));
            apply_common_mcp(&mut body, &args);

            client.post("elements", Value::Object(body))
        }
        "elements_add_widget" => {
            let data_id = args.get("data_id").and_then(Value::as_u64);
            let raw_data = args
                .get("raw_data")
                .and_then(Value::as_str)
                .map(String::from);
            let width = args
                .get("width")
                .and_then(Value::as_i64)
                .ok_or_else(|| anyhow::anyhow!("missing width"))?;
            let height = args
                .get("height")
                .and_then(Value::as_i64)
                .ok_or_else(|| anyhow::anyhow!("missing height"))?;

            if data_id.is_none() && raw_data.is_none() {
                bail!("provide either data_id or raw_data");
            }

            let mut widget_group = Map::new();
            if let Some(d) = data_id {
                widget_group.insert("elementDataId".into(), d.into());
            }
            if let Some(r) = raw_data {
                widget_group.insert("rawData".into(), r.into());
            }
            widget_group.insert("width".into(), width.into());
            widget_group.insert("height".into(), height.into());

            let mut body = Map::new();
            body.insert("type".into(), "widget".into());
            body.insert("widget".into(), Value::Object(widget_group));
            apply_common_mcp(&mut body, &args);

            client.post("elements", Value::Object(body))
        }
        "elements_add_media" => {
            let source = args
                .get("source")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow::anyhow!("missing source"))?;
            let volume = args.get("volume").and_then(Value::as_i64);
            let width = args.get("width").and_then(Value::as_i64);
            let height = args.get("height").and_then(Value::as_i64);
            let autoplay = args
                .get("autoplay")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let r#loop = args.get("loop").and_then(Value::as_bool).unwrap_or(false);
            let timestamp = args.get("timestamp").and_then(Value::as_i64);

            let mut media_group = Map::new();
            media_group.insert("source".into(), source.into());
            if let Some(v) = volume {
                media_group.insert("volume".into(), v.into());
            }
            if let Some(w) = width {
                media_group.insert("width".into(), w.into());
            }
            if let Some(h) = height {
                media_group.insert("height".into(), h.into());
            }
            if autoplay {
                media_group.insert("autoplay".into(), true.into());
            }
            if r#loop {
                media_group.insert("loop".into(), true.into());
            }
            if let Some(t) = timestamp {
                media_group.insert("timestamp".into(), t.into());
            }

            let mut body = Map::new();
            body.insert("type".into(), "media".into());
            body.insert("media".into(), Value::Object(media_group));
            apply_common_mcp(&mut body, &args);

            client.post("elements", Value::Object(body))
        }
        "elements_update" => {
            let id = args
                .get("id")
                .and_then(Value::as_u64)
                .ok_or_else(|| anyhow::anyhow!("missing id"))?;
            let mut body = Map::new();
            apply_common_mcp(&mut body, &args);
            set_mcp(&mut body, "locked", args.get("locked").cloned());
            set_mcp(
                &mut body,
                "alwaysLoaded",
                args.get("always_loaded").cloned(),
            );
            set_mcp(&mut body, "indexLock", args.get("index_lock").cloned());

            let mut text = Map::new();
            set_mcp(&mut text, "text", args.get("text").cloned());
            set_mcp(&mut text, "size", args.get("text_size").cloned());
            set_mcp(&mut text, "color", args.get("color").cloned());
            set_mcp(&mut text, "font", args.get("font").cloned());
            set_mcp(&mut text, "css", args.get("css").cloned());
            if !text.is_empty() {
                body.insert("text".into(), Value::Object(text));
            }

            let mut image = Map::new();
            set_mcp(
                &mut image,
                "elementDataId",
                args.get("image_data_id").cloned(),
            );
            set_mcp(&mut image, "url", args.get("image_url").cloned());
            set_mcp(&mut image, "width", args.get("image_width").cloned());
            set_mcp(&mut image, "height", args.get("image_height").cloned());
            if !image.is_empty() {
                body.insert("image".into(), Value::Object(image));
            }

            let mut widget = Map::new();
            set_mcp(
                &mut widget,
                "elementDataId",
                args.get("widget_data_id").cloned(),
            );
            set_mcp(&mut widget, "rawData", args.get("widget_raw_data").cloned());
            set_mcp(&mut widget, "width", args.get("widget_width").cloned());
            set_mcp(&mut widget, "height", args.get("widget_height").cloned());
            if !widget.is_empty() {
                body.insert("widget".into(), Value::Object(widget));
            }

            let mut media = Map::new();
            set_mcp(&mut media, "source", args.get("media_source").cloned());
            set_mcp(&mut media, "volume", args.get("media_volume").cloned());
            set_mcp(&mut media, "playing", args.get("media_playing").cloned());
            set_mcp(
                &mut media,
                "timestamp",
                args.get("media_timestamp").cloned(),
            );
            set_mcp(&mut media, "autoplay", args.get("media_autoplay").cloned());
            set_mcp(&mut media, "loop", args.get("media_loop").cloned());
            set_mcp(&mut media, "width", args.get("media_width").cloned());
            set_mcp(&mut media, "height", args.get("media_height").cloned());
            if !media.is_empty() {
                body.insert("media".into(), Value::Object(media));
            }

            if body.is_empty() {
                bail!("provide at least one field to update");
            }
            client.patch(&format!("elements?id={id}"), Value::Object(body))
        }
        "elementdata_list" => {
            let id = args.get("id").and_then(Value::as_u64);
            let asset_name = args.get("name").and_then(Value::as_str);
            let folder = args.get("folder").and_then(Value::as_u64);
            let query = qs(&[
                ("id", id.map(|v| v.to_string())),
                ("name", asset_name.map(String::from)),
                ("folder", folder.map(|v| v.to_string())),
            ]);
            client.get(&format!("elementdata{query}"))
        }
        "elementdata_add" => {
            let asset_name = args
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow::anyhow!("missing name"))?;
            let kind_str = args
                .get("type")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow::anyhow!("missing type"))?;
            let data = args
                .get("data")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow::anyhow!("missing data"))?;
            let width = args
                .get("width")
                .and_then(Value::as_i64)
                .ok_or_else(|| anyhow::anyhow!("missing width"))?;
            let height = args
                .get("height")
                .and_then(Value::as_i64)
                .ok_or_else(|| anyhow::anyhow!("missing height"))?;
            let folder_id = args.get("folder_id").and_then(Value::as_u64);

            let mut body = Map::new();
            body.insert("name".into(), asset_name.into());
            body.insert("type".into(), kind_str.into());
            body.insert("data".into(), data.into());
            body.insert("width".into(), width.into());
            body.insert("height".into(), height.into());
            if let Some(fid) = folder_id {
                body.insert("folderId".into(), fid.into());
            }
            client.post("elementdata", Value::Object(body))
        }
        "elementdata_update" => {
            let id = args
                .get("id")
                .and_then(Value::as_u64)
                .ok_or_else(|| anyhow::anyhow!("missing id"))?;
            let mut body = Map::new();
            set_mcp(&mut body, "name", args.get("name").cloned());
            set_mcp(&mut body, "data", args.get("data").cloned());
            set_mcp(&mut body, "width", args.get("width").cloned());
            set_mcp(&mut body, "height", args.get("height").cloned());
            set_mcp(&mut body, "folderId", args.get("folder_id").cloned());
            if body.is_empty() {
                bail!("provide at least one field to update");
            }
            client.patch(&format!("elementdata?id={id}"), Value::Object(body))
        }
        "elementdata_delete" => {
            let id = args.get("id").and_then(Value::as_u64);
            let asset_name = args.get("name").and_then(Value::as_str);
            if id.is_none() && asset_name.is_none() {
                bail!("provide either id or name");
            }
            let query = qs(&[
                ("id", id.map(|v| v.to_string())),
                ("name", asset_name.map(String::from)),
            ]);
            client.delete(&format!("elementdata{query}"))
        }
        "folders_list" => client.get("folders"),
        "folders_add" => {
            let folder_name = args
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow::anyhow!("missing name"))?;
            let icon = args.get("icon").and_then(Value::as_str).map(String::from);
            let mut body = Map::new();
            body.insert("name".into(), folder_name.into());
            if let Some(i) = icon {
                body.insert("icon".into(), i.into());
            }
            client.post("folders", Value::Object(body))
        }
        "folders_update" => {
            let id = args
                .get("id")
                .and_then(Value::as_u64)
                .ok_or_else(|| anyhow::anyhow!("missing id"))?;
            let mut body = Map::new();
            set_mcp(&mut body, "name", args.get("name").cloned());
            set_mcp(&mut body, "icon", args.get("icon").cloned());
            if body.is_empty() {
                bail!("provide name and/or icon");
            }
            client.patch(&format!("folders?id={id}"), Value::Object(body))
        }
        "folders_delete" => {
            let id = args
                .get("id")
                .and_then(Value::as_u64)
                .ok_or_else(|| anyhow::anyhow!("missing id"))?;
            let no_preserve_elements = args
                .get("no_preserve_elements")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let query = qs(&[
                ("id", Some(id.to_string())),
                (
                    "preserveElements",
                    no_preserve_elements.then(|| "false".to_string()),
                ),
            ]);
            client.delete(&format!("folders{query}"))
        }
        _ => bail!("tool not found: {name}"),
    }
}

fn apply_common_mcp(body: &mut Map<String, Value>, args: &Value) {
    set_mcp(body, "x", args.get("x").cloned());
    set_mcp(body, "y", args.get("y").cloned());
    set_mcp(body, "transform", args.get("transform").cloned());
    set_mcp(body, "transparency", args.get("transparency").cloned());
    set_mcp(body, "clip", args.get("clip").cloned());
    set_mcp(body, "layoutId", args.get("layout_id").cloned());
}

fn set_mcp(map: &mut Map<String, Value>, key: &str, value: Option<Value>) {
    if let Some(v) = value {
        if !v.is_null() {
            map.insert(key.to_string(), v);
        }
    }
}

fn get_tools_list() -> Value {
    json!([
        {
            "name": "ping",
            "description": "Check that the overlay API is reachable",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "overlay": {
                        "type": "string",
                        "description": "Optional overlay profile nickname"
                    }
                }
            }
        },
        {
            "name": "whoami",
            "description": "Show the identity and permissions behind the configured token",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "overlay": {
                        "type": "string",
                        "description": "Optional overlay profile nickname"
                    }
                }
            }
        },
        {
            "name": "layouts_list",
            "description": "List layouts",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "overlay": {
                        "type": "string",
                        "description": "Optional overlay profile nickname"
                    }
                }
            }
        },
        {
            "name": "layouts_add",
            "description": "Create a layout",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Name of the new layout"
                    },
                    "active": {
                        "type": "boolean",
                        "description": "Make it the active layout immediately"
                    },
                    "overlay": {
                        "type": "string",
                        "description": "Optional overlay profile nickname"
                    }
                },
                "required": ["name"]
            }
        },
        {
            "name": "layouts_duplicate",
            "description": "Duplicate a layout and its elements",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": {
                        "type": "integer",
                        "description": "ID of the layout to duplicate"
                    },
                    "overlay": {
                        "type": "string",
                        "description": "Optional overlay profile nickname"
                    }
                },
                "required": ["id"]
            }
        },
        {
            "name": "layouts_rename",
            "description": "Rename a layout",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": {
                        "type": "integer",
                        "description": "ID of the layout to rename"
                    },
                    "name": {
                        "type": "string",
                        "description": "New name for the layout"
                    },
                    "overlay": {
                        "type": "string",
                        "description": "Optional overlay profile nickname"
                    }
                },
                "required": ["id", "name"]
            }
        },
        {
            "name": "layouts_delete",
            "description": "Delete a layout",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": {
                        "type": "integer",
                        "description": "ID of the layout to delete"
                    },
                    "preserve_elements": {
                        "type": "boolean",
                        "description": "Move the layout's elements instead of deleting them"
                    },
                    "preserve_layout_id": {
                        "type": "integer",
                        "description": "Layout ID to move preserved elements to (defaults to 1)"
                    },
                    "overlay": {
                        "type": "string",
                        "description": "Optional overlay profile nickname"
                    }
                },
                "required": ["id"]
            }
        },
        {
            "name": "layouts_set_active",
            "description": "Set the active layout (scene switch). Provide either id or name.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": {
                        "type": "integer",
                        "description": "ID of the layout to make active"
                    },
                    "name": {
                        "type": "string",
                        "description": "Name of the layout to make active"
                    },
                    "overlay": {
                        "type": "string",
                        "description": "Optional overlay profile nickname"
                    }
                }
            }
        },
        {
            "name": "elements_list",
            "description": "List elements",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": {
                        "type": "integer",
                        "description": "Fetch a single element by ID"
                    },
                    "layout": {
                        "type": "integer",
                        "description": "Filter elements by layout ID"
                    },
                    "overlay": {
                        "type": "string",
                        "description": "Optional overlay profile nickname"
                    }
                }
            }
        },
        {
            "name": "elements_delete",
            "description": "Delete an element",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": {
                        "type": "integer",
                        "description": "ID of the element to delete"
                    },
                    "overlay": {
                        "type": "string",
                        "description": "Optional overlay profile nickname"
                    }
                },
                "required": ["id"]
            }
        },
        {
            "name": "elements_add_text",
            "description": "Add a text element to a layout",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "text": { "type": "string", "description": "Text content" },
                    "size": { "type": "integer", "description": "Font size (default 48)" },
                    "color": { "type": "string", "description": "Text color in hex (default #ffffff)" },
                    "font": { "type": "string", "description": "Font family name" },
                    "css": { "type": "string", "description": "Extra CSS applied to the element" },
                    "x": { "type": "integer", "description": "X coordinate" },
                    "y": { "type": "integer", "description": "Y coordinate" },
                    "transform": { "type": "string", "description": "CSS transform string (overrides x/y)" },
                    "transparency": { "type": "integer", "description": "Opacity 0-100" },
                    "clip": { "type": "string", "description": "CSS clip-path string" },
                    "layout_id": { "type": "integer", "description": "Target layout ID (defaults to active layout)" },
                    "overlay": { "type": "string", "description": "Optional overlay profile nickname" }
                },
                "required": ["text"]
            }
        },
        {
            "name": "elements_add_image",
            "description": "Add an image element from an asset or raw URL",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "data_id": { "type": "integer", "description": "Existing element data (asset) ID" },
                    "url": { "type": "string", "description": "Raw image URL (requires width and height)" },
                    "width": { "type": "integer", "description": "Width in pixels" },
                    "height": { "type": "integer", "description": "Height in pixels" },
                    "x": { "type": "integer", "description": "X coordinate" },
                    "y": { "type": "integer", "description": "Y coordinate" },
                    "transform": { "type": "string", "description": "CSS transform string (overrides x/y)" },
                    "transparency": { "type": "integer", "description": "Opacity 0-100" },
                    "clip": { "type": "string", "description": "CSS clip-path string" },
                    "layout_id": { "type": "integer", "description": "Target layout ID (defaults to active layout)" },
                    "overlay": { "type": "string", "description": "Optional overlay profile nickname" }
                }
            }
        },
        {
            "name": "elements_add_widget",
            "description": "Add a widget element",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "data_id": { "type": "integer", "description": "Existing element data (asset) ID" },
                    "raw_data": { "type": "string", "description": "Inline widget definition" },
                    "width": { "type": "integer", "description": "Width in pixels" },
                    "height": { "type": "integer", "description": "Height in pixels" },
                    "x": { "type": "integer", "description": "X coordinate" },
                    "y": { "type": "integer", "description": "Y coordinate" },
                    "transform": { "type": "string", "description": "CSS transform string (overrides x/y)" },
                    "transparency": { "type": "integer", "description": "Opacity 0-100" },
                    "clip": { "type": "string", "description": "CSS clip-path string" },
                    "layout_id": { "type": "integer", "description": "Target layout ID (defaults to active layout)" },
                    "overlay": { "type": "string", "description": "Optional overlay profile nickname" }
                },
                "required": ["width", "height"]
            }
        },
        {
            "name": "elements_add_media",
            "description": "Add a media (video/audio) element",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": { "type": "string", "description": "Media source URL (e.g. YouTube link)" },
                    "volume": { "type": "integer", "description": "Volume 0-100 (default 100)" },
                    "width": { "type": "integer", "description": "Width in pixels" },
                    "height": { "type": "integer", "description": "Height in pixels" },
                    "autoplay": { "type": "boolean", "description": "Start playing immediately" },
                    "loop": { "type": "boolean", "description": "Loop playback" },
                    "timestamp": { "type": "integer", "description": "Start position in seconds" },
                    "x": { "type": "integer", "description": "X coordinate" },
                    "y": { "type": "integer", "description": "Y coordinate" },
                    "transform": { "type": "string", "description": "CSS transform string (overrides x/y)" },
                    "transparency": { "type": "integer", "description": "Opacity 0-100" },
                    "clip": { "type": "string", "description": "CSS clip-path string" },
                    "layout_id": { "type": "integer", "description": "Target layout ID (defaults to active layout)" },
                    "overlay": { "type": "string", "description": "Optional overlay profile nickname" }
                },
                "required": ["source"]
            }
        },
        {
            "name": "elements_update",
            "description": "Update fields on an element",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "integer", "description": "ID of the element to update" },
                    "x": { "type": "integer", "description": "X coordinate" },
                    "y": { "type": "integer", "description": "Y coordinate" },
                    "transform": { "type": "string", "description": "CSS transform string" },
                    "transparency": { "type": "integer", "description": "Opacity 0-100" },
                    "clip": { "type": "string", "description": "CSS clip-path string" },
                    "layout_id": { "type": "integer", "description": "Move to layout ID" },
                    "locked": { "type": "boolean", "description": "Lock or unlock element" },
                    "always_loaded": { "type": "boolean", "description": "Keep loaded in background" },
                    "index_lock": { "type": "boolean", "description": "Lock z-index layer" },
                    "text": { "type": "string", "description": "New text content (text elements)" },
                    "text_size": { "type": "integer", "description": "New font size" },
                    "color": { "type": "string", "description": "New color" },
                    "font": { "type": "string", "description": "New font" },
                    "css": { "type": "string", "description": "New custom CSS" },
                    "image_data_id": { "type": "integer", "description": "Swap image asset to data ID" },
                    "image_url": { "type": "string", "description": "Swap to image URL" },
                    "image_width": { "type": "integer", "description": "New image width" },
                    "image_height": { "type": "integer", "description": "New image height" },
                    "widget_data_id": { "type": "integer", "description": "Swap widget asset to data ID" },
                    "widget_raw_data": { "type": "string", "description": "Replace widget inline definition" },
                    "widget_width": { "type": "integer", "description": "New widget width" },
                    "widget_height": { "type": "integer", "description": "New widget height" },
                    "media_source": { "type": "string", "description": "New media source URL" },
                    "media_volume": { "type": "integer", "description": "New media volume 0-100" },
                    "media_playing": { "type": "boolean", "description": "Play or pause media" },
                    "media_timestamp": { "type": "integer", "description": "Seek position in seconds" },
                    "media_autoplay": { "type": "boolean", "description": "Autoplay media" },
                    "media_loop": { "type": "boolean", "description": "Loop media" },
                    "media_width": { "type": "integer", "description": "New media width" },
                    "media_height": { "type": "integer", "description": "New media height" },
                    "overlay": { "type": "string", "description": "Optional overlay profile nickname" }
                },
                "required": ["id"]
            }
        },
        {
            "name": "elementdata_list",
            "description": "List element data assets",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "integer", "description": "Fetch a single asset by ID" },
                    "name": { "type": "string", "description": "Filter by exact name" },
                    "folder": { "type": "integer", "description": "Filter by folder ID" },
                    "overlay": { "type": "string", "description": "Optional overlay profile nickname" }
                }
            }
        },
        {
            "name": "elementdata_add",
            "description": "Create an element data asset",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Name of the asset" },
                    "type": { "type": "string", "enum": ["image", "widget", "media", "text"], "description": "Asset type" },
                    "data": { "type": "string", "description": "URL or asset data string" },
                    "width": { "type": "integer", "description": "Asset width" },
                    "height": { "type": "integer", "description": "Asset height" },
                    "folder_id": { "type": "integer", "description": "Folder ID to place asset in" },
                    "overlay": { "type": "string", "description": "Optional overlay profile nickname" }
                },
                "required": ["name", "type", "data", "width", "height"]
            }
        },
        {
            "name": "elementdata_update",
            "description": "Update fields on an element data asset",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "integer", "description": "ID of the asset to update" },
                    "name": { "type": "string", "description": "New name of the asset" },
                    "data": { "type": "string", "description": "New URL or data string" },
                    "width": { "type": "integer", "description": "New asset width" },
                    "height": { "type": "integer", "description": "New asset height" },
                    "folder_id": { "type": "integer", "description": "Move asset to folder ID" },
                    "overlay": { "type": "string", "description": "Optional overlay profile nickname" }
                },
                "required": ["id"]
            }
        },
        {
            "name": "elementdata_delete",
            "description": "Delete an asset (also deletes elements that reference it). Provide either id or name.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "integer", "description": "ID of the asset to delete" },
                    "name": { "type": "string", "description": "Name of the asset to delete" },
                    "overlay": { "type": "string", "description": "Optional overlay profile nickname" }
                }
            }
        },
        {
            "name": "folders_list",
            "description": "List folders",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "overlay": { "type": "string", "description": "Optional overlay profile nickname" }
                }
            }
        },
        {
            "name": "folders_add",
            "description": "Create a folder",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Name of the folder" },
                    "icon": { "type": "string", "description": "Emoji or icon identifier" },
                    "overlay": { "type": "string", "description": "Optional overlay profile nickname" }
                },
                "required": ["name"]
            }
        },
        {
            "name": "folders_update",
            "description": "Update a folder's name and/or icon",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "integer", "description": "ID of the folder to update" },
                    "name": { "type": "string", "description": "New name of the folder" },
                    "icon": { "type": "string", "description": "New icon" },
                    "overlay": { "type": "string", "description": "Optional overlay profile nickname" }
                },
                "required": ["id"]
            }
        },
        {
            "name": "folders_delete",
            "description": "Delete a folder",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "integer", "description": "ID of the folder to delete" },
                    "no_preserve_elements": { "type": "boolean", "description": "Also delete all assets inside the folder" },
                    "overlay": { "type": "string", "description": "Optional overlay profile nickname" }
                },
                "required": ["id"]
            }
        }
    ])
}
