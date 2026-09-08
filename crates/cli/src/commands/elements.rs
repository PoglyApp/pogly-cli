use anyhow::{bail, Result};
use serde_json::{Map, Value};

use crate::api::client::qs;
use crate::api::types::Element;
use crate::cli::{AddElement, CommonElementArgs, ElementsSub, GlobalArgs, UpdateElement};
use crate::output;

pub fn run(cmd: ElementsSub, global: &GlobalArgs) -> Result<()> {
    match cmd {
        ElementsSub::List { id, layout, raw } => list(global, id, layout, raw),
        ElementsSub::Add(add) => create(global, add),
        ElementsSub::Update(update) => patch(global, update),
        ElementsSub::Delete { id } => delete(global, id),
    }
}

fn list(global: &GlobalArgs, id: Option<u32>, layout: Option<u32>, raw: bool) -> Result<()> {
    let query = qs(&[
        ("id", id.map(|v| v.to_string())),
        ("layout", layout.map(|v| v.to_string())),
        ("raw", raw.then(|| "true".to_string())),
    ]);
    let response = super::client_for(global)?.get(&format!("elements{query}"))?;
    if global.json {
        output::print_json(&response);
        return Ok(());
    }
    let elements: Vec<Element> = serde_json::from_value(response["elements"].clone())?;
    if elements.is_empty() {
        println!("No elements found.");
        return Ok(());
    }
    if id.is_some() && elements.len() == 1 {
        print_detail(&elements[0]);
        return Ok(());
    }
    let rows: Vec<Vec<String>> = elements
        .iter()
        .map(|e| {
            vec![
                e.id.to_string(),
                e.kind.clone(),
                e.x.to_string(),
                e.y.to_string(),
                e.layout_id.to_string(),
                e.locked.to_string(),
                output::truncate(&e.summary(), 40),
            ]
        })
        .collect();
    output::table(
        &["ID", "TYPE", "X", "Y", "LAYOUT", "LOCKED", "SUMMARY"],
        &rows,
    );
    Ok(())
}

fn print_detail(element: &Element) {
    println!("Element {} ({})", element.id, element.kind);
    println!("  Layout:   {}", element.layout_id);
    println!("  Position: {}, {}", element.x, element.y);
    println!("  Locked:   {}", element.locked);
    if let Some(t) = &element.text {
        println!("  Text:     {}", output::truncate(&t.text, 60));
        println!(
            "  Style:    size {}, color {}, font '{}'",
            t.size, t.color, t.font
        );
        if !t.css.is_empty() {
            println!("  CSS:      {}", output::truncate(&t.css, 60));
        }
    }
    if let Some(i) = &element.image {
        match (&i.url, i.element_data_id) {
            (Some(url), _) => println!("  Image:    {url}"),
            (None, Some(data_id)) => println!("  Image:    data #{data_id}"),
            _ => {}
        }
        println!("  Size:     {}x{}", i.width, i.height);
    }
    if let Some(w) = &element.widget {
        println!("  Widget:   data #{}", w.element_data_id);
        println!("  Size:     {}x{}", w.width, w.height);
        match w.raw_data.as_deref() {
            Some("") => println!("  Raw data: (empty, uses the linked element data)"),
            Some(raw) => println!("  Raw data: {}", output::truncate(raw, 60)),
            None => {}
        }
    }
    if let Some(m) = &element.media {
        println!("  Source:   {}", m.source);
        println!("  Size:     {}x{}", m.width, m.height);
        println!("  Volume:   {}", m.volume);
        println!(
            "  Playing:  {} (autoplay {}, loop {}, timestamp {}s)",
            m.playing, m.autoplay, m.loop_, m.timestamp
        );
    }
}

fn create(global: &GlobalArgs, add: AddElement) -> Result<()> {
    let body = build_add_body(add)?;
    let response = super::client_for(global)?.post("elements", Value::Object(body))?;
    if global.json {
        output::print_json(&response);
        return Ok(());
    }
    let element = &response["element"];
    println!(
        "Created element {} ({}) on layout {}",
        element["id"],
        element["type"].as_str().unwrap_or("?"),
        element["layoutId"]
    );
    Ok(())
}

fn patch(global: &GlobalArgs, update: UpdateElement) -> Result<()> {
    let body = build_patch_body(&update)?;
    let response = super::client_for(global)?
        .patch(&format!("elements?id={}", update.id), Value::Object(body))?;
    if global.json {
        output::print_json(&response);
        return Ok(());
    }
    println!("OK");
    Ok(())
}

fn delete(global: &GlobalArgs, id: u32) -> Result<()> {
    let response = super::client_for(global)?.delete(&format!("elements?id={id}"))?;
    if global.json {
        output::print_json(&response);
        return Ok(());
    }
    println!("OK");
    Ok(())
}

fn set(map: &mut Map<String, Value>, key: &str, value: Option<impl Into<Value>>) {
    if let Some(v) = value {
        map.insert(key.to_string(), v.into());
    }
}

fn apply_common(body: &mut Map<String, Value>, common: &CommonElementArgs) {
    set(body, "x", common.x);
    set(body, "y", common.y);
    set(body, "transform", common.transform.clone());
    set(body, "transparency", common.transparency);
    set(body, "clip", common.clip.clone());
    set(body, "layoutId", common.layout_id);
}

fn build_add_body(add: AddElement) -> Result<Map<String, Value>> {
    let mut body = Map::new();
    match add {
        AddElement::Text {
            text,
            size,
            color,
            font,
            css,
            common,
        } => {
            body.insert("type".into(), "text".into());
            apply_common(&mut body, &common);
            let mut group = Map::new();
            group.insert("text".into(), text.into());
            set(&mut group, "size", size);
            set(&mut group, "color", color);
            set(&mut group, "font", font);
            set(&mut group, "css", css);
            body.insert("text".into(), Value::Object(group));
        }
        AddElement::Image {
            data_id,
            url,
            width,
            height,
            common,
        } => {
            if url.is_some() && (width.is_none() || height.is_none()) {
                bail!("--url requires --width and --height");
            }
            body.insert("type".into(), "image".into());
            apply_common(&mut body, &common);
            let mut group = Map::new();
            set(&mut group, "elementDataId", data_id);
            set(&mut group, "url", url);
            set(&mut group, "width", width);
            set(&mut group, "height", height);
            body.insert("image".into(), Value::Object(group));
        }
        AddElement::Widget {
            data_id,
            raw_data,
            width,
            height,
            common,
        } => {
            body.insert("type".into(), "widget".into());
            apply_common(&mut body, &common);
            let mut group = Map::new();
            set(&mut group, "elementDataId", data_id);
            set(&mut group, "rawData", raw_data);
            group.insert("width".into(), width.into());
            group.insert("height".into(), height.into());
            body.insert("widget".into(), Value::Object(group));
        }
        AddElement::Media {
            source,
            volume,
            width,
            height,
            autoplay,
            r#loop,
            timestamp,
            common,
        } => {
            body.insert("type".into(), "media".into());
            apply_common(&mut body, &common);
            let mut group = Map::new();
            group.insert("source".into(), source.into());
            set(&mut group, "volume", volume);
            set(&mut group, "width", width);
            set(&mut group, "height", height);
            if autoplay {
                group.insert("autoplay".into(), true.into());
            }
            if r#loop {
                group.insert("loop".into(), true.into());
            }
            set(&mut group, "timestamp", timestamp);
            body.insert("media".into(), Value::Object(group));
        }
    }
    Ok(body)
}

fn build_patch_body(update: &UpdateElement) -> Result<Map<String, Value>> {
    let mut body = Map::new();
    apply_common(&mut body, &update.common);
    set(&mut body, "locked", update.locked);
    set(&mut body, "alwaysLoaded", update.always_loaded);
    set(&mut body, "indexLock", update.index_lock);

    let mut text = Map::new();
    set(&mut text, "text", update.text.clone());
    set(&mut text, "size", update.text_size);
    set(&mut text, "color", update.color.clone());
    set(&mut text, "font", update.font.clone());
    set(&mut text, "css", update.css.clone());
    if !text.is_empty() {
        body.insert("text".into(), Value::Object(text));
    }

    let mut image = Map::new();
    set(&mut image, "elementDataId", update.image_data_id);
    set(&mut image, "url", update.image_url.clone());
    set(&mut image, "width", update.image_width);
    set(&mut image, "height", update.image_height);
    if !image.is_empty() {
        body.insert("image".into(), Value::Object(image));
    }

    let mut widget = Map::new();
    set(&mut widget, "elementDataId", update.widget_data_id);
    set(&mut widget, "rawData", update.widget_raw_data.clone());
    set(&mut widget, "width", update.widget_width);
    set(&mut widget, "height", update.widget_height);
    if !widget.is_empty() {
        body.insert("widget".into(), Value::Object(widget));
    }

    let mut media = Map::new();
    set(&mut media, "source", update.media_source.clone());
    set(&mut media, "volume", update.media_volume);
    set(&mut media, "playing", update.media_playing);
    set(&mut media, "timestamp", update.media_timestamp);
    set(&mut media, "autoplay", update.media_autoplay);
    set(&mut media, "loop", update.media_loop);
    set(&mut media, "width", update.media_width);
    set(&mut media, "height", update.media_height);
    if !media.is_empty() {
        body.insert("media".into(), Value::Object(media));
    }

    if body.is_empty() {
        bail!("provide at least one field to update");
    }
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn patch_body_contains_only_supplied_fields() {
        let update = UpdateElement {
            id: 5,
            common: CommonElementArgs {
                x: Some(100),
                ..Default::default()
            },
            media_volume: Some(50),
            ..Default::default()
        };
        let body = build_patch_body(&update).unwrap();
        assert_eq!(
            Value::Object(body),
            json!({"x": 100, "media": {"volume": 50}})
        );
    }

    #[test]
    fn empty_patch_is_rejected() {
        let update = UpdateElement::default();
        assert!(build_patch_body(&update).is_err());
    }

    #[test]
    fn add_text_body_shape() {
        let body = build_add_body(AddElement::Text {
            text: "GG".into(),
            size: Some(72),
            color: None,
            font: None,
            css: None,
            common: CommonElementArgs {
                x: Some(10),
                y: Some(20),
                ..Default::default()
            },
        })
        .unwrap();
        assert_eq!(
            Value::Object(body),
            json!({"type": "text", "x": 10, "y": 20, "text": {"text": "GG", "size": 72}})
        );
    }

    #[test]
    fn add_image_url_requires_dimensions() {
        let result = build_add_body(AddElement::Image {
            data_id: None,
            url: Some("https://cdn.7tv.app/emote/x/4x.webp".into()),
            width: Some(64),
            height: None,
            common: CommonElementArgs::default(),
        });
        assert!(result.is_err());
    }

    #[test]
    fn tristate_bools_serialize_false() {
        let update = UpdateElement {
            id: 1,
            locked: Some(false),
            media_playing: Some(false),
            ..Default::default()
        };
        let body = build_patch_body(&update).unwrap();
        assert_eq!(
            Value::Object(body),
            json!({"locked": false, "media": {"playing": false}})
        );
    }
}
